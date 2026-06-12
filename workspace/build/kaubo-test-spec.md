# Kaubo 测试用例规格

每条用例定义：输入 → Lexer 预期 → Parser 预期 → CodeGen 预期 → VM 预期。
按 Phase 组织，每个 Phase 的用例集构成该阶段的验收标准。

---

## Phase 1 测试：Lexer

### L01 — 空文件

```
输入: ""
预期 tokens: [EOF]
```

### L02 — 单关键字

```
输入: "var"
预期 tokens: [VAR("var", 1:1), EOF(1:4)]
```

### L03 — 标识符 vs 关键字

```
输入: "var x = varName"
预期 tokens: [VAR, ID("x"), OP("="), ID("varName"), EOF]
// varName 是标识符，不是关键字 VAR
```

### L04 — 整数和浮点数

```
输入: "42 3.14"
预期 tokens: [INT("42"), FLOAT("3.14"), EOF]
```

### L05 — 浮点数边界情况

```
输入: "3."
预期 tokens: [INT("3"), DOT, EOF]
// 3. 不是浮点数——小数点后必须有数字

输入: ".5"
预期 tokens: [DOT, INT("5"), EOF]
// .5 不是浮点数——小数点前必须有数字
```

### L06 — 字符串（双引号和单引号）

```
输入: '"hello" \'world\''
预期 tokens: [STRING("hello"), STRING("world"), EOF]
```

### L07 — 字符串转义

```
输入: '"line1\\nline2\\tindented"'
预期 tokens: [STRING("line1\nline2\tindented"), EOF]
```

### L08 — 运算符和双字符运算符

```
输入: "+ - * / % = == != < > <= >= ->"
预期 tokens: [
  PLUS, MINUS, ASTERISK, SLASH, PERCENT,
  EQUAL, DOUBLE_EQUAL, EXCLAMATION_EQUAL,
  LESS_THAN, GREATER_THAN, LESS_THAN_EQUAL, GREATER_THAN_EQUAL,
  FAT_ARROW,
  EOF
]
```

### L09 — 分隔符

```
输入: "( ) { } [ ] ; , . : |"
预期 tokens: [
  LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET,
  SEMI, COMMA, DOT, COLON, PIPE,
  EOF
]
```

### L10 — 行注释

```
输入: "var x = 1; // this is a comment\nvar y = 2;"
预期 tokens: [VAR, ID("x"), OP("="), INT("1"), SEMI, VAR, ID("y"), OP("="), INT("2"), SEMI, EOF]
// 注释和其中的内容不出现在 token 流中
```

### L11 — 块注释

```
输入: "var /* inline */ x = /* multi\nline */ 1;"
预期 tokens: [VAR, ID("x"), OP("="), INT("1"), SEMI, EOF]
```

### L12 — 完整的代码行

```
输入: 'var add = |a, b| { return a + b; };'
预期 tokens: [
  VAR, ID("add"), OP("="), PIPE, ID("a"), COMMA, ID("b"), PIPE,
  LBRACE, RETURN, ID("a"), OP("+"), ID("b"), SEMI, RBRACE, SEMI, EOF
]
```

### L13 — JSON 关键字后跟花括号

```
输入: 'json { name: "Alice" }'
预期 tokens: [JSON, LBRACE, ID("name"), COLON, STRING("Alice"), RBRACE, EOF]
```

### L14 — 错误：未终止字符串

```
输入: '"unterminated
预期: LexError(UnterminatedString)
// 但 lexer 不崩溃——返回 partial_tokens + errors
```

### L15 — 错误：非法字符

```
输入: "var x = @;"
预期: 扫描到 @ 时产生 InvalidChar 错误 token，继续扫描后续字符
```

---

## Phase 2 测试：Parser

### P01 — 空模块

```
输入: ""
预期 AST: Module { statements: [] }
```

### P02 — 变量声明（无类型标注）

```
输入: "var x = 42;"
预期 AST:
  Module
    VarDecl { name: "x", type_annotation: null, initializer: LiteralInt(42), is_public: false }
```

### P03 — 变量声明（带类型标注）

```
输入: "var x: int = 42;"
预期 AST:
  Module
    VarDecl { name: "x", type_annotation: Named("int"), initializer: LiteralInt(42) }
```

### P04 — pub var

```
输入: "pub var PI = 3.14;"
预期 AST:
  Module
    VarDecl { name: "PI", initializer: LiteralFloat(3.14), is_public: true }
```

### P05 — 运算符优先级：乘法优先于加法

```
输入: "1 + 2 * 3;"
预期 AST:
  ExprStmt
    Binary { op: Plus,
      left: LiteralInt(1),
      right: Binary { op: Asterisk, left: LiteralInt(2), right: LiteralInt(3) }
    }
// 关键：* 在 + 的下方，所以 2*3 先算
```

### P06 — 运算符优先级：括号覆盖

```
输入: "(1 + 2) * 3;"
预期 AST:
  ExprStmt
    Binary { op: Asterisk,
      left: Grouping(Binary { op: Plus, LiteralInt(1), LiteralInt(2) }),
      right: LiteralInt(3)
    }
// 关键：括号让 + 先算
```

### P07 — 赋值是右结合

```
输入: "a = b = 42;"
预期 AST:
  ExprStmt
    Binary { op: Equal,
      left: VarRef("a"),
      right: Binary { op: Equal, left: VarRef("b"), right: LiteralInt(42) }
    }
// b = 42 先求值，结果赋给 a
```

### P08 — 成员访问和下标

```
输入: "obj.field[0];"
预期 AST:
  ExprStmt
    IndexAccess {
      object: MemberAccess { object: VarRef("obj"), member: "field" },
      index: LiteralInt(0)
    }
```

### P09 — 函数调用

```
输入: "print(1, 2, 3);"
预期 AST:
  ExprStmt
    FunctionCall {
      function_expr: VarRef("print"),
      arguments: [LiteralInt(1), LiteralInt(2), LiteralInt(3)]
    }
```

### P10 — 列表字面量

```
输入: "[1, 2, 3];"
预期 AST:
  ExprStmt
    LiteralList { elements: [LiteralInt(1), LiteralInt(2), LiteralInt(3)] }
```

### P11 — 空列表

```
输入: "[];"
预期 AST:
  ExprStmt
    LiteralList { elements: [] }
```

### P12 — JSON 字面量

```
输入: 'json { name: "Alice", age: 30 };'
预期 AST:
  ExprStmt
    JsonLiteral {
      entries: [("name", LiteralString("Alice")), ("age", LiteralInt(30))]
    }
```

### P13 — 无参 lambda

```
输入: "var f = || { return 42; };"
预期 AST:
  VarDecl { name: "f",
    initializer: Lambda { params: [], return_type: null, body: Block([ReturnStmt(LiteralInt(42))]) }
  }
```

### P14 — 有参 lambda（带类型标注和返回类型）

```
输入: "var add = |a: int, b: int| -> int { return a + b; };"
预期 AST:
  VarDecl { name: "add",
    initializer: Lambda {
      params: [("a", Named("int")), ("b", Named("int"))],
      return_type: Named("int"),
      body: Block([ReturnStmt(Binary { op: Plus, VarRef("a"), VarRef("b") })])
    }
  }
```

### P15 — if 语句（完整形式）

```
输入: |
  if x > 5 {
      print(1);
  } elif x > 0 {
      print(2);
  } else {
      print(3);
  }
预期 AST:
  Module
    IfStmt {
      if_condition: Binary { op: GreaterThan, VarRef("x"), LiteralInt(5) },
      then_body: Block([PrintStmt(LiteralInt(1))]),
      elif_conditions: [Binary { op: GreaterThan, VarRef("x"), LiteralInt(0) }],
      elif_bodies: [Block([PrintStmt(LiteralInt(2))])],
      else_body: Some(Block([PrintStmt(LiteralInt(3))]))
    }
```

### P16 — while 循环

```
输入: |
  while x > 0 {
      x = x - 1;
  }
预期 AST:
  Module
    WhileStmt {
      condition: Binary { op: GreaterThan, VarRef("x"), LiteralInt(0) },
      body: Block([ExprStmt(Binary { op: Equal, VarRef("x"), Binary { op: Minus, VarRef("x"), LiteralInt(1) } })])
    }
```

### P17 — for 循环

```
输入: |
  for var item in list {
      print(item);
  }
预期 AST:
  Module
    ForStmt {
      iterator: VarRef("item"),
      iterable: VarRef("list"),
      body: Block([PrintStmt(VarRef("item"))])
    }
```

### P18 — struct 定义

```
输入: |
  struct Point {
      x: int,
      y: int
  }
预期 AST:
  Module
    StructStmt {
      name: "Point",
      fields: [FieldDef { name: "x", type_annotation: Named("int") },
               FieldDef { name: "y", type_annotation: Named("int") }]
    }
```

### P19 — struct 字面量

```
输入: "Point { x: 1, y: 2 };"
预期 AST:
  ExprStmt
    StructLiteral {
      name: "Point",
      fields: [("x", LiteralInt(1)), ("y", LiteralInt(2))]
    }
// 注意：Point 必须大写开头
```

### P20 — impl + operator 重载

```
输入: |
  impl Vector {
      operator add: |self, other| -> Vector {
          return Vector { x: self.x + other.x, y: self.y + other.y };
      }
  }
预期 AST:
  Module
    ImplStmt {
      struct_name: "Vector",
      methods: [
        MethodDef {
          name: "operator add",
          lambda: Lambda {
            params: [("self", null), ("other", null)],
            return_type: Named("Vector"),
            body: Block([ReturnStmt(StructLiteral { name: "Vector", ... })])
          }
        }
      ]
    }
```

### P21 — import 语句

```
输入: |
  import math;
  from std import print, assert;
预期 AST:
  Module [
    ImportStmt { module_path: "math", items: [], alias: null },
    ImportStmt { module_path: "std", items: ["print", "assert"], alias: null }
  ]
```

### P22 — 嵌套块

```
输入: |
  {
      var x = 1;
      {
          var y = 2;
          print(x + y);
      }
  }
预期 AST:
  Module
    Block [
      VarDecl { name: "x", initializer: LiteralInt(1) },
      Block [
        VarDecl { name: "y", initializer: LiteralInt(2) },
        PrintStmt(Binary { op: Plus, VarRef("x"), VarRef("y") })
      ]
    ]
```

### P23 — 错误：缺少右括号

```
输入: "print(1, 2;"
预期: ParseError { kind: MissingRightParen }
```

### P24 — 错误：意外的 token

```
输入: "var = 42;"
预期: ParseError { kind: ExpectedIdentifier, found: "=" }
// var 后面应该是标识符，不是 =
```

### P25 — 错误：module 关键字已废弃

```
输入: "module foo;"
预期: ParseError { kind: ModuleKeywordDeprecated }
```

---

## Phase 3 测试：CodeGen

### C01 — 整数常量

```
输入 AST: LiteralInt(42)
预期 Chunk:
  code: [LOAD_CONST(0)]
  constants: [Int(42)]
```

### C02 — 加法

```
输入 AST: Binary { op: Plus, LiteralInt(1), LiteralInt(2) }
预期 Chunk:
  code: [LOAD_CONST(0), LOAD_CONST(1), ADD, RETURN_VALUE]
  constants: [Int(1), Int(2)]
```

### C03 — 表达式含乘法优先级

```
输入 AST: Binary { op: Plus, LiteralInt(1),
                   Binary { op: Asterisk, LiteralInt(2), LiteralInt(3) } }
预期 Chunk:
  code: [
    LOAD_CONST(0),    // 1
    LOAD_CONST(1),    // 2
    LOAD_CONST(2),    // 3
    MUL,
    ADD,
    RETURN_VALUE
  ]
  constants: [Int(1), Int(2), Int(3)]
  // 关键: 2 和 3 先入栈，MUL 先执行，然后 ADD
```

### C04 — 变量声明 + 引用

```
输入 AST:
  Module [
    VarDecl { name: "x", initializer: LiteralInt(42) },
    PrintStmt(VarRef("x"))
  ]
预期 Chunk:
  - STORE_NAME("x") 对应 var x = 42
  - LOAD_NAME("x") 对应 print(x)
  - CALL("print") 对应 print(...)
```

### C05 — if 语句

```
输入 AST: If { condition: VarRef("x"),
               then_body: Block([PrintStmt(LiteralInt(1))]),
               else_body: null }
预期 Chunk:
  code: [
    LOAD_NAME("x"),           // 加载条件
    JUMP_IF_FALSE(offset),    // 为假跳转到 then 之后
    LOAD_CONST("1"),          // then body: print(1)
    CALL("print"),
    // (JUMP_IF_FALSE 的目标地址)
    ...
  ]
```

### C06 — while 循环

```
输入 AST: While { condition: VarRef("x"),
                  body: Block([ExprStmt(Binary { op: Equal, VarRef("x"),
                                        Binary { op: Minus, VarRef("x"), Int(1) }) }]) }
预期 Chunk:
  code: [
    // (循环头: JUMP_IF_FALSE 的目标)
    LOAD_NAME("x"),           // 条件
    JUMP_IF_FALSE(end),       // 为假跳出
    LOAD_NAME("x"),           // body: x = x - 1
    LOAD_CONST(1),
    SUB,
    STORE_NAME("x"),
    JUMP_BACK(start),         // 跳回循环头
    // (end: JUMP_IF_FALSE 的目标)
    ...
  ]
```

### C07 — lambda

```
输入 AST: Lambda { params: [("a"), ("b")],
                   return_type: null,
                   body: Block([ReturnStmt(Binary { op: Plus, VarRef("a"), VarRef("b") })]) }
预期 Chunk:
  - 函数体编译为独立的 Chunk (child_chunk)
  - 外层: MAKE_FUNCTION(child_chunk_index)  → 把函数对象压栈
```

### C08 — lambda 调用

```
输入 AST: FunctionCall {
  function_expr: Lambda { ... },
  arguments: [LiteralInt(2), LiteralInt(3)]
}
预期 Chunk:
  - 先编译 Lambda，MAKE_FUNCTION
  - 编译参数: LOAD_CONST(2), LOAD_CONST(3)
  - CALL(2)  // argc = 2
```

### C09 — 列表字面量

```
输入 AST: LiteralList { elements: [LiteralInt(1), LiteralInt(2), LiteralInt(3)] }
预期 Chunk:
  code: [
    LOAD_CONST(1), LOAD_CONST(2), LOAD_CONST(3),
    BUILD_LIST(3),   // 从栈顶取 3 个元素构建列表
    RETURN_VALUE
  ]
```

### C10 — 错误：未定义变量

```
输入 AST: PrintStmt(VarRef("undefined_var"))
// 在此之前没有 STORE_NAME("undefined_var")
预期: CodeGenError { type: "UninitializedVariable", name: "undefined_var" }
```

---

## Phase 4 测试：VM

### V01 — Hello World

```
输入: print("Hello, Kaubo!");
预期 VM: stdout = "Hello, Kaubo\n", exit_code = 0
```

### V02 — 整数运算

```
输入: print(1 + 2 * 3);
预期 VM: stdout = "7\n"
轨迹: 2*3=6, 1+6=7
```

### V03 — 浮点数运算

```
输入: print(3.14 * 2.0);
预期 VM: stdout = "6.28\n"
```

### V04 — 字符串拼接

```
输入: print("Hello, " + "World!");
预期 VM: stdout = "Hello, World!\n"
```

### V05 — 变量存储和读取

```
输入: |
  var x = 42;
  var y = x + 1;
  print(y);
预期 VM: stdout = "43\n"
```

### V06 — 布尔和比较

```
输入: |
  print(1 < 2);
  print(3 == 4);
  print(true and false);
  print(true or false);
  print(not true);
预期 VM:
  stdout = "true\nfalse\nfalse\ntrue\nfalse\n"
```

### V07 — if / elif / else

```
输入: |
  var x = 10;
  if x > 20 {
      print("big");
  } elif x > 5 {
      print("medium");
  } else {
      print("small");
  }
预期 VM: stdout = "medium\n"
```

### V08 — if 条件为假不进入 then

```
输入: |
  if false {
      print("nope");
  }
  print("yes");
预期 VM: stdout = "yes\n"
```

### V09 — while 循环

```
输入: |
  var x = 3;
  while x > 0 {
      print(x);
      x = x - 1;
  }
预期 VM: stdout = "3\n2\n1\n"
```

### V10 — for 循环

```
输入: |
  var sum = 0;
  for var item in [1, 2, 3, 4, 5] {
      sum = sum + item;
  }
  print(sum);
预期 VM: stdout = "15\n"
```

### V11 — lambda（无参数）

```
输入: |
  var f = || { return 42; };
  print(f());
预期 VM: stdout = "42\n"
```

### V12 — lambda（带参数）

```
输入: |
  var add = |a, b| { return a + b; };
  print(add(2, 3));
预期 VM: stdout = "5\n"
```

### V13 — lambda 闭包

```
输入: |
  var makeCounter = || {
      var count = 0;
      return || {
          count = count + 1;
          return count;
      };
  };
  var counter = makeCounter();
  print(counter());
  print(counter());
  print(counter());
预期 VM: stdout = "1\n2\n3\n"
// count 是 makeCounter 的局部变量，内部 lambda 捕获了它 (upvalue)
```

### V14 — return（隐式返回）

```
输入: |
  var f = || { 42; };
  print(f());
预期 VM: stdout = "42\n"
// 函数最后一个表达式的值是返回值，不需要写 return
```

### V15 — 列表字面量和下标

```
输入: |
  var list = [10, 20, 30];
  print(list[0]);
  print(list[1]);
  print(list[2]);
预期 VM: stdout = "10\n20\n30\n"
```

### V16 — 列表赋值

```
输入: |
  var list = [1, 2, 3];
  list[0] = 99;
  print(list[0]);
预期 VM: stdout = "99\n"
```

### V17 — JSON 字面量和成员访问

```
输入: |
  var p = json { name: "Alice", age: 30 };
  print(p.name);
  print(p.age);
预期 VM: stdout = "Alice\n30\n"
```

### V18 — 内置函数：len

```
输入: |
  print(len([1, 2, 3]));
  print(len("hello"));
预期 VM: stdout = "3\n5\n"
```

### V19 — 内置函数：type

```
输入: |
  print(type(42));
  print(type("hello"));
  print(type(true));
  print(type([1, 2]));
预期 VM: stdout = "int\nstring\nbool\nlist\n"
```

### V20 — 递归

```
输入: |
  var fib = |n| {
      if n <= 1 {
          return n;
      }
      return fib(n - 1) + fib(n - 2);
  };
  print(fib(10));
预期 VM: stdout = "55\n"
```

### V21 — 错误：除零

```
输入: |
  print(1 / 0);
预期 VM: success: false, stderr 包含 "DivisionByZero"
```

### V22 — 错误：未定义变量

```
输入: |
  print(undefined_var);
预期 VM: success: false, stderr 包含 "UndefinedVariable"
```

---

## Phase 5 集成测试

### INT01 — 全流水线：Source → Tokens → AST → Bytecode → Result

```
输入源码:
  var x = 10;
  var y = 20;
  print(x + y);

验证每个阶段的输出符合预期:
  Lexer:   15 个 token (VAR ID OP INT SEMI VAR ID OP INT SEMI PRINT LPAREN ID OP ID RPAREN SEMI EOF)
  Parser:  Module 包含 2 个 VarDecl + 1 个 PrintStmt
  CodeGen: Chunk 包含 STORE_NAME(x) STORE_NAME(y) LOAD_NAME(x) LOAD_NAME(y) ADD CALL(print)
  VM:      stdout = "30\n"
```

### INT02 — 完整计算器

```
输入源码:
  var a = 10;
  var b = 3;
  print(a + b);
  print(a - b);
  print(a * b);
  print(a / b);

预期 VM:
  stdout = "13\n7\n30\n3.3333333333333335\n"
```

### INT03 — FizzBuzz

```
输入源码:
  var i = 1;
  while i <= 15 {
      if i % 15 == 0 {
          print("FizzBuzz");
      } elif i % 3 == 0 {
          print("Fizz");
      } elif i % 5 == 0 {
          print("Buzz");
      } else {
          print(i);
      }
      i = i + 1;
  }

预期 VM: stdout 的前 15 行 =
  1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz\n
```

---

## 测试用例汇总

| Phase | 用例数 | 覆盖内容 |
|-------|--------|---------|
| Phase 1 (Lexer) | 15 | 所有 token 类型、注释、转义、错误恢复 |
| Phase 2 (Parser) | 25 | 所有语句、表达式优先级、lambda、struct/impl、错误 |
| Phase 3 (CodeGen) | 10 | 每类 AST 节点的指令序列、常量池 |
| Phase 4 (VM) | 22 | 所有运行时语义、闭包、递归、错误 |
| Phase 5 (集成) | 3 | 端到端全流水线 |
| **合计** | **75** | |
