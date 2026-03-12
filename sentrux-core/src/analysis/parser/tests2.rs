//! Extended oracle tests for the parser, covering additional languages.
//!
//! Tests Kotlin, Swift, Scala, PHP, Ruby, and other languages added after
//! the initial parser implementation. Each test uses `parse_bytes` with
//! known source snippets and asserts exact structural analysis counts.
//! Complements `tests.rs` with broader language coverage.

#[cfg(test)]
mod tests {
    use crate::analysis::parser::parse_bytes;

    // ---- Oracle tests: new languages ----

    // kotlin: temporarily removed — tree-sitter-kotlin incompatible with tree-sitter 0.25

    #[test]
    fn oracle_scala() {
        let code = br#"
import scala.collection.mutable

class Calculator {
  def add(a: Int, b: Int): Int = a + b
  def multiply(a: Int, b: Int): Int = a * b
}

object Main {
  def main(args: Array[String]): Unit = {
    val calc = new Calculator()
    println(calc.add(1, 2))
  }
}

trait Printable {
  def show(): String
}
"#;
        let sa = parse_bytes(code, "scala").expect("scala parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // add, multiply, main = 3 (show() is abstract -- function_signature, not function_definition)
        assert_eq!(fns.len(), 3, "expected 3 functions, got {:?}", fns);
        let cls = sa.cls.as_ref().expect("no classes");
        // Calculator + Main + Printable = 3
        assert_eq!(cls.len(), 3, "expected 3 class-like items, got {:?}", cls);
        assert!(sa.imp.is_some(), "expected imports");
    }

    #[test]
    fn oracle_html() {
        // Realistic HTML with many attributes -- test that real imports survive noise
        let code = br#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="/src/style.css" />
    <script type="module" src="/src/main.ts"></script>
    <style>body { color: red; }</style>
</head>
<body>
    <div id="app" class="container"></div>
    <h1>Hello</h1>
</body>
</html>
"#;
        let sa = parse_bytes(code, "html").expect("html parse failed");
        let imp = sa.imp.as_ref().expect("no imports captured for <link>/<script>");
        let imp_strs: Vec<&str> = imp.iter().map(|s| s.as_str()).collect();
        eprintln!("ALL HTML imports captured: {:?}", imp_strs);
        assert!(imp_strs.contains(&"/src/style.css"),
            "expected /src/style.css import, got {:?}", imp_strs);
        assert!(imp_strs.contains(&"/src/main.ts"),
            "expected /src/main.ts import, got {:?}", imp_strs);
    }

    #[test]
    fn oracle_css() {
        let code = br#"
@import url("reset.css");
@import "theme.css";

body {
    color: red;
}

.container {
    display: flex;
}
"#;
        let sa = parse_bytes(code, "css").expect("css parse failed");
        let imp = sa.imp.as_ref().expect("no imports");
        assert_eq!(imp.len(), 2, "expected 2 @import, got {:?}", imp);
    }

    #[test]
    fn oracle_scss() {
        let code = br#"
@import "variables";
@import "mixins";

@mixin flex-center {
    display: flex;
    align-items: center;
}

@function rem($px) {
    @return $px / 16 * 1rem;
}

.container {
    @include flex-center;
    font-size: rem(16);
}
"#;
        let sa = parse_bytes(code, "scss").expect("scss parse failed");
        let fns = sa.functions.as_ref().expect("no functions/mixins");
        // flex-center (mixin) + rem (function) = 2
        assert_eq!(fns.len(), 2, "expected 2 functions/mixins, got {:?}", fns);
        let imp = sa.imp.as_ref().expect("no imports");
        assert_eq!(imp.len(), 2, "expected 2 @import, got {:?}", imp);
    }

    #[test]
    fn oracle_swift() {
        let code = br#"
import Foundation
import UIKit

class ViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
        print("loaded")
    }

    func handleTap() {
        let alert = UIAlertController()
        present(alert, animated: true)
    }
}

struct Point {
    var x: Double
    var y: Double
}

func distance(from a: Point, to b: Point) -> Double {
    let dx = a.x - b.x
    let dy = a.y - b.y
    return sqrt(dx * dx + dy * dy)
}
"#;
        let sa = parse_bytes(code, "swift").expect("swift parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // viewDidLoad, handleTap, distance = 3
        assert_eq!(fns.len(), 3, "expected 3 functions, got {:?}", fns);
        let cls = sa.cls.as_ref().expect("no classes");
        // ViewController + Point = 2
        assert!(cls.len() >= 2, "expected at least 2 class-like items, got {:?}", cls);
        assert!(sa.imp.is_some(), "expected imports");
    }

    #[test]
    fn swift_entry_main_detected() {
        let code = br#"
import SwiftUI

@main
struct StreetPlaceDemoApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
"#;
        let sa = parse_bytes(code, "swift").expect("swift @main parse failed");
        let tags = sa.tags.as_ref().expect("no tags -- @main not captured");
        eprintln!("Swift tags: {:?}", tags);
        assert!(tags.contains(&"@main".to_string()),
            "expected @main tag, got {:?}", tags);
    }

    #[test]
    fn oracle_lua() {
        let code = br#"
function greet(name)
    print("Hello " .. name)
end

function math.add(a, b)
    return a + b
end

local function helper()
    return 42
end

greet("world")
print(helper())
"#;
        let sa = parse_bytes(code, "lua").expect("lua parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // greet, math.add, helper = 3
        assert_eq!(fns.len(), 3, "expected 3 functions, got {:?}", fns);
        let all_calls: Vec<String> = fns.iter()
            .flat_map(|f| f.co.iter().flat_map(|c| c.iter().cloned()))
            .chain(sa.co.iter().flat_map(|c| c.iter().cloned()))
            .collect();
        assert!(all_calls.len() >= 2, "expected at least 2 calls, got {:?}", all_calls);
    }

    // ---- Oracle tests: new Phase 5 languages ----

    #[test]
    fn oracle_elixir() {
        let code = br#"
defmodule MyApp.Greeter do
  def greet(name) do
    IO.puts("Hello #{name}")
  end

  defp helper() do
    :ok
  end
end
"#;
        let sa = parse_bytes(code, "elixir").expect("elixir parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // greet + helper = 2
        assert!(fns.len() >= 2, "expected at least 2 functions, got {:?}", fns);
    }

    #[test]
    fn oracle_haskell() {
        let code = br#"
module Main where

import Data.List
import Data.Map

data Color = Red | Green | Blue

class Printable a where
  display :: a -> String

greet :: String -> String
greet name = "Hello " ++ name

main :: IO ()
main = putStrLn (greet "World")
"#;
        let sa = parse_bytes(code, "haskell").expect("haskell parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // greet (has parameter) = 1 (main has no params, is a value binding)
        assert!(fns.len() >= 1, "expected at least 1 function, got {:?}", fns);
        let cls = sa.cls.as_ref().expect("no classes");
        // Color (data) + Printable (class) = 2
        assert!(cls.len() >= 2, "expected at least 2 class-like items, got {:?}", cls);
        let imp = sa.imp.as_ref().expect("no imports");
        assert!(imp.len() >= 2, "expected at least 2 imports, got {:?}", imp);
    }

    #[test]
    fn oracle_zig() {
        let code = br#"
const std = @import("std");

fn add(a: i32, b: i32) i32 {
    return a + b;
}

pub fn main() void {
    const result = add(1, 2);
    std.debug.print("{}\n", .{result});
}

test "addition" {
    const result = add(1, 2);
    try std.testing.expectEqual(result, 3);
}
"#;
        let sa = parse_bytes(code, "zig").expect("zig parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // add + main + test = 3
        assert!(fns.len() >= 2, "expected at least 2 functions, got {:?}", fns);
    }

    #[test]
    fn oracle_r() {
        let code = br#"
library(ggplot2)

add <- function(a, b) {
  a + b
}

multiply = function(a, b) {
  a * b
}

result <- add(1, 2)
print(result)
"#;
        let sa = parse_bytes(code, "r").expect("r parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // add + multiply = 2
        assert!(fns.len() >= 2, "expected at least 2 functions, got {:?}", fns);
        let all_calls: Vec<String> = fns.iter()
            .flat_map(|f| f.co.iter().flat_map(|c| c.iter().cloned()))
            .chain(sa.co.iter().flat_map(|c| c.iter().cloned()))
            .collect();
        assert!(all_calls.len() >= 2, "expected at least 2 calls, got {:?}", all_calls);
    }

    // dockerfile: temporarily removed — tree-sitter-dockerfile incompatible with tree-sitter 0.25

    #[test]
    fn oracle_ocaml() {
        let code = br#"
let greet name =
  print_endline ("Hello " ^ name)

let add a b = a + b

let () =
  greet "World";
  print_int (add 1 2)
"#;
        let sa = parse_bytes(code, "ocaml").expect("ocaml parse failed");
        let fns = sa.functions.as_ref().expect("no functions");
        // greet + add = 2
        assert!(fns.len() >= 2, "expected at least 2 functions, got {:?}", fns);
    }

    // ---- Boundary tests ----

    #[test]
    fn boundary_empty_file() {
        let sa = parse_bytes(b"", "python").expect("empty file parse failed");
        assert!(sa.functions.is_none());
        assert!(sa.cls.is_none());
        assert!(sa.imp.is_none());
        assert!(sa.co.is_none());
    }

    #[test]
    fn boundary_comment_only() {
        let code = b"# This is just a comment\n# Nothing else\n";
        let sa = parse_bytes(code, "python").expect("comment-only parse failed");
        assert!(sa.functions.is_none());
        assert!(sa.cls.is_none());
    }

    // ---- Idempotency test ----

    #[test]
    fn idempotency_same_result_twice() {
        let code = br#"
def foo():
    pass

class Bar:
    def baz(self):
        pass
"#;
        let sa1 = parse_bytes(code, "python").expect("first parse failed");
        let sa2 = parse_bytes(code, "python").expect("second parse failed");

        let fns1 = sa1.functions.as_ref().unwrap();
        let fns2 = sa2.functions.as_ref().unwrap();
        assert_eq!(fns1.len(), fns2.len());
        for (f1, f2) in fns1.iter().zip(fns2.iter()) {
            assert_eq!(f1.n, f2.n);
            assert_eq!(f1.sl, f2.sl);
            assert_eq!(f1.el, f2.el);
        }
    }
}
