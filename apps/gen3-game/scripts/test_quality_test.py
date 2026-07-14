import unittest
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_quality import declaration_lines


class DeclarationLineTests(unittest.TestCase):
    def test_excludes_complete_use_struct_and_enum_declarations_only(self) -> None:
        source = [
            (1, "use std::{"),
            (2, "    fs,"),
            (3, "    path::Path,"),
            (4, "};"),
            (5, "pub struct Runtime {"),
            (6, "    state: usize,"),
            (7, "}"),
            (8, "struct Marker("),
            (9, "    usize,"),
            (10, ");"),
            (11, "pub enum Event {"),
            (12, "    Started,"),
            (13, "    Failed(String),"),
            (14, "}"),
            (15, "impl Runtime {"),
            (16, "    fn run(&mut self) {}"),
            (17, "}"),
        ]

        self.assertEqual(declaration_lines(source), set(range(1, 15)))


if __name__ == "__main__":
    unittest.main()
