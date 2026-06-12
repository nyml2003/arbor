# Kaubo 解析器规格

手写递归下降 + Pratt 表达式解析。不描述文法，描述解析算法。可以直接对着这份文档写 `parser.rs`。

---

## 词法 (Lexer)

解析器的输入是 `Token[]`。Token 类型定义见 `kaubo-wasm-types.ts`。

---

## 1. parse_module

```
fn parse_module(tokens, pos):
    statements = []
    while peek() != EOF:
        stmt = parse_statement(tokens, pos)
        if stmt is Ok: statements.push(stmt)
        else:          记录错误，尝试同步到下一个分号/关键字，继续
        match_token(SEMICOLON)  // 语句间可选分号
    return Module { statements }
```

错误恢复策略：遇到 ParseError 后，跳过 token 直到遇到 `;` 或关键字（`var if while for return struct impl import pub print`），然后继续解析。

---

## 2. parse_statement — token 分派

```
fn parse_statement(tokens, pos):
    match current token:
        VAR        → parse_var_decl()
        PUB        → parse_pub_var_decl()      // 先消费 pub，然后 expect(var)，再 parse_var_decl_body
        IF         → parse_if_stmt()
        WHILE      → parse_while_stmt()
        FOR        → parse_for_stmt()
        RETURN     → parse_return_stmt()
        PRINT      → parse_print_stmt()
        IMPORT     → parse_import_stmt()
        FROM       → parse_import_stmt()        // from ... import ...
        STRUCT     → parse_struct_def()
        IMPL       → parse_impl_def()
        LBRACE     → parse_block()
        SEMICOLON  → EmptyStmt
        MODULE     → error("module keyword deprecated")
        其他       → parse_expression_statement()
```

**注意**：`PRINT` 不是关键字级特殊处理——`print` 在 lexer 里是关键字 token，但解析时作为语句处理。表达式语句是 fallback：如果当前 token 不匹配任何已知语句关键字，就当作表达式解析。

---

## 3. parse_expression — Pratt 解析器

```
fn parse_expression(tokens, pos, min_precedence = 0):
    left = parse_unary(tokens, pos)            // 前缀

    loop:
        op = peek()
        prec = precedence_of(op)
        if prec < min_precedence: break         // 优先级不够，停止爬升

        if is_right_associative(op):            // 只有 = 是右结合
            next_min = prec
        else:
            next_min = prec + 1                 // 左结合：更高优先级才能抢

        consume(op)
        right = parse_expression(tokens, pos, next_min)
        left = Binary { left, op, right }

    return left
```

### 优先级表

```
Token                   优先级    结合性
EQUAL (=)                  50    右
OR                         60    左
AND                        80    左
DOUBLE_EQUAL, NE,          100    左
  LT, GT, LTE, GTE
PLUS, MINUS                200    左
ASTERISK, SLASH, PERCENT   300    左
DOT                        400    左    ← 成员访问，但实际在 parse_postfix 处理
```

**关键细节**：
- `=` 是右结合：`a = b = 42` → `a = (b = 42)`
- `+` `-` 是左结合：`a + b + c` → `(a + b) + c`
- 比较运算符**不可链式**：`a < b < c` 解析为 `(a < b) < c`（结果可能不是你想要的，但这就是当前语法）
- `NOT` 不在这里——它是一元前缀，在 `parse_unary` 处理
- `PIPE` 不在这里——`|` 在 kaubo 中只用作 lambda 分隔符，不是二元运算符
- `DOT` 在表中但不在这里处理——成员访问在 `parse_postfix` 的后缀循环中处理

---

## 4. parse_unary

```
fn parse_unary(tokens, pos):
    match current token:
        MINUS →  consume; operand = parse_unary(); return Unary { op: MINUS, operand }
        NOT   →  consume; operand = parse_unary(); return Unary { op: NOT, operand }
        其他   →  return parse_postfix(parse_primary(tokens, pos), tokens, pos)
```

`-` 和 `not` 递归调用 `parse_unary`，所以 `- -x` 和 `not not x` 都能正确解析。

---

## 5. parse_primary

```
fn parse_primary(tokens, pos):
    match current token:
        INT    → LiteralInt(value)
        FLOAT  → LiteralFloat(value)
        STRING → LiteralString(value)
        TRUE   → LiteralTrue
        FALSE  → LiteralFalse
        NULL   → LiteralNull

        ID  →
            if peek_next() == LBRACE && is_uppercase(current.value):
                parse_struct_literal()           // Point { x: 1 }
            else:
                VarRef(name)

        LPAREN →
            consume; expr = parse_expression(); expect(RPAREN); Grouping(expr)

        LBRACKET →
            parse_list_literal()                 // [1, 2, 3]

        PIPE →
            parse_lambda()                       // |a, b| { ... }

        JSON →
            parse_json_literal()                 // json { k: v }

        _ → error("unexpected token")
```

**struct 字面量识别规则**：`ID` + `LBRACE` → 如果 ID 是大写开头（`A-Z`），解析为 `StructLiteral`；否则解析为 `VarRef`，让后续的 `{` 成为 block 开始（可能出错，但这就是当前行为）。

---

## 6. parse_postfix — 后缀循环

```
fn parse_postfix(expr, tokens, pos):
    loop:
        match peek():
            DOT →
                consume; name = expect(ID); expr = MemberAccess { object: expr, member: name }
            LPAREN →
                args = parse_argument_list(); expr = FunctionCall { function_expr: expr, arguments: args }
            LBRACKET →
                consume; index = parse_expression(); expect(RBRACKET); expr = IndexAccess { object: expr, index }
            AS →
                consume; ty = parse_type(); expr = As { expr, target_type: ty }
            _ → break
    return expr
```

后缀按此顺序循环。`a.b(c)[0] as int` → 成员访问 → 函数调用 → 下标 → 类型转换。

---

## 7. 语句解析函数

### parse_var_decl

```
fn parse_var_decl(is_public = false):
    expect(VAR); name = expect(ID);
    type_ann = if peek() == COLON { consume; parse_type() } else { null }
    expect(EQUAL); value = parse_expression();
    expect(SEMICOLON);
    return VarDecl { name, type_annotation, initializer: value, is_public }
```

### parse_if_stmt

```
fn parse_if_stmt():
    expect(IF); condition = parse_expression(); then_body = parse_block();
    elif_conds = []; elif_bodies = [];
    while peek() == ELIF:
        consume; elif_conds.push(parse_expression()); elif_bodies.push(parse_block());
    else_body = if peek() == ELSE { consume; parse_block() } else { null }
    return IfStmt { if_condition: condition, then_body, elif_conditions: elif_conds, elif_bodies, else_body }
```

注意：条件**不需要**括号。body **必须**是花括号块。

### parse_while_stmt

```
fn parse_while_stmt():
    expect(WHILE); condition = parse_expression(); body = parse_block();
    return WhileStmt { condition, body }
```

### parse_for_stmt

```
fn parse_for_stmt():
    expect(FOR); expect(VAR); iterator = expect(ID);
    expect(IN); iterable = parse_expression(); body = parse_block();
    return ForStmt { iterator: VarRef(iterator), iterable, body }
```

### parse_return_stmt / parse_print_stmt

```
fn parse_return_stmt():
    expect(RETURN);
    value = if peek() != SEMICOLON { parse_expression() } else { null }
    expect(SEMICOLON);
    return ReturnStmt { value }

fn parse_print_stmt():
    expect(PRINT); expr = parse_expression(); expect(SEMICOLON);
    return PrintStmt { expression: expr }
```

### parse_block

```
fn parse_block():
    expect(LBRACE); stmts = [];
    while peek() != RBRACE && peek() != EOF:
        stmts.push(parse_statement());
        match_token(SEMICOLON);
    expect(RBRACE);
    return Block { statements: stmts }
```

### parse_import_stmt

```
fn parse_import_stmt():
    if peek() == IMPORT:
        consume; path = parse_path(); alias = if peek() == AS { consume; expect(ID) } else { null };
        expect(SEMICOLON);
        return ImportStmt { module_path: path, items: [], alias }
    if peek() == FROM:
        consume; path = parse_path(); expect(IMPORT);
        items = [];
        items.push(expect(ID));  // print 也是有效的导入项名
        while peek() == COMMA: consume; items.push(expect(ID));
        expect(SEMICOLON);
        return ImportStmt { module_path: path, items, alias: null }

fn parse_path():
    path = expect(ID);
    while peek() == DOT: consume; path += "." + expect(ID);
    return path
```

### parse_struct_def

```
fn parse_struct_def():
    expect(STRUCT); name = expect(ID); expect(LBRACE);
    fields = [];
    fields.push(FieldDef { name: expect(ID), expect(COLON); type_annotation: parse_type() });
    while peek() == COMMA: consume; fields.push(...);
    expect(RBRACE);
    return StructStmt { name, fields }
```

### parse_impl_def

```
fn parse_impl_def():
    expect(IMPL); struct_name = expect(ID); expect(LBRACE);
    methods = [];
    loop:
        method_name = if peek() == OPERATOR { consume; "operator " + expect(ID) } else { expect(ID) };
        expect(COLON); lambda = parse_lambda();
        methods.push(MethodDef { name: method_name, lambda });
        if peek() != COMMA: break; consume;
    expect(RBRACE);
    return ImplStmt { struct_name, methods }
```

---

## 8. 其他解析函数

### parse_lambda

```
fn parse_lambda():
    expect(PIPE);  // 开 |
    params = [];
    if peek() != PIPE:
        loop:
            name = expect(ID);
            ty = if peek() == COLON { consume; parse_type() } else { null }
            params.push((name, ty));
            if peek() != COMMA: break; consume;
    expect(PIPE);  // 闭 |
    return_ty = if peek() == FAT_ARROW { consume; parse_type() } else { null }
    body = parse_block();
    return Lambda { params, return_type: return_ty, body }
```

### parse_type

```
fn parse_type():
    match peek():
        ID →
            name = consume();
            if peek() == LT:
                consume; args = [parse_type()]; while peek() == COMMA: consume; args.push(parse_type());
                expect(GT);
                return GenericType { name, type_args: args }
            return NamedType { name }
        PIPE →
            consume; params = [parse_type()];
            while peek() == COMMA: consume; params.push(parse_type());
            expect(PIPE); return_ty = null;
            if peek() == FAT_ARROW: consume; return_ty = parse_type();
            return FunctionType { params, return_type: return_ty }
```

支持的类型：`int`, `float`, `string`, `bool`, `List<int>`, `Tuple<int, string>`, `|int, int| -> bool`

### parse_list_literal

```
fn parse_list_literal():
    expect(LBRACKET); elements = [];
    if peek() != RBRACKET:
        elements.push(parse_expression());
        while peek() == COMMA: consume; elements.push(parse_expression());
    expect(RBRACKET);
    return LiteralList { elements }
```

### parse_json_literal

```
fn parse_json_literal():
    expect(JSON); expect(LBRACE); entries = [];
    if peek() != RBRACE:
        key = if peek() == STRING { consume(); strip_quotes(value) } else { expect(ID) };
        expect(COLON); value = parse_expression();
        entries.push((key, value));
        while peek() == COMMA: consume;
            key = ...; value = ...; entries.push((key, value));
    expect(RBRACE);
    return JsonLiteral { entries }
```

### parse_struct_literal

```
fn parse_struct_literal():
    name = expect(ID); expect(LBRACE); fields = [];
    if peek() != RBRACE:
        field_name = expect(ID); expect(COLON); value = parse_expression();
        fields.push((field_name, value));
        while peek() == COMMA: consume;
            field_name = ...; value = ...; fields.push(...);
    expect(RBRACE);
    return StructLiteral { name, fields }
```

### parse_argument_list

```
fn parse_argument_list():
    expect(LPAREN); args = [];
    if peek() != RPAREN:
        args.push(parse_expression());
        while peek() == COMMA: consume; args.push(parse_expression());
    expect(RPAREN);
    return args
```

---

## 辅助函数

```
fn peek()        → tokens[pos].kind
fn peek_next()   → tokens[pos+1].kind
fn consume()     → t = tokens[pos]; pos++; return t
fn expect(kind)  → if peek() == kind { consume() } else { return ParseError(UnexpectedToken) }
fn match_token(kind) → if peek() == kind { consume() }
fn is_uppercase(s)   → s[0] in 'A'..'Z'
```

---

## MVP 收录 vs 不做

同之前的表格。语法从源码提取，只收录已验证可用的特性。不收录保留关键字和死代码路径。
