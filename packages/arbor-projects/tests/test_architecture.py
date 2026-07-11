from pathlib import Path
import ast
import unittest


class ProductionConstantTests(unittest.TestCase):
    def test_function_bodies_do_not_contain_literal_constants(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "arbor_projects"
        violations: list[str] = []

        for source in source_root.rglob("*.py"):
            tree = ast.parse(source.read_text(encoding="utf-8"), filename=str(source))
            for function in (
                node
                for node in ast.walk(tree)
                if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            ):
                docstring = (
                    function.body[0]
                    if function.body
                    and isinstance(function.body[0], ast.Expr)
                    and isinstance(function.body[0].value, ast.Constant)
                    and isinstance(function.body[0].value.value, str)
                    else None
                )
                for node in ast.walk(function):
                    if not isinstance(node, ast.Constant):
                        continue
                    if (
                        node.value is None
                        or node.value is True
                        or node.value is False
                        or node.value is Ellipsis
                    ):
                        continue
                    if docstring is not None and node is docstring.value:
                        continue
                    violations.append(
                        f"{source.relative_to(source_root)}:"
                        f"{node.lineno}:{function.name}:{node.value!r}"
                    )

        self.assertEqual(violations, [])


if __name__ == "__main__":
    unittest.main()
