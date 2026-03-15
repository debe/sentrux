//! Diagnostic test: dumps full tree-sitter AST for import statements across languages.
//!
//! Run with: cargo test ast_import_dump -- --ignored --nocapture
//! This prints the exact node structure tree-sitter produces, so we can build
//! a generic AST walker that extracts module paths WITHOUT text re-parsing.

#[cfg(test)]
mod tests {
    use crate::analysis::lang_registry;
    use tree_sitter::Parser;

    /// Sample import statements per language.
    #[allow(unused)]
    const SAMPLES: &[(&str, &str)] = &[
        (
            "python",
            r#"import os
import os.path
from collections import OrderedDict
from ..utils import helper
from typing import Protocol, ABC
"#,
        ),
        (
            "rust",
            r#"use std::collections::HashMap;
use crate::models::{episode::{Episode, Injection}, primitive};
mod graph;
pub use self::graph::compute_levels;
"#,
        ),
        (
            "go",
            r#"package main
import (
    "fmt"
    "os"
    cfg "github.com/user/repo/config"
    _ "github.com/lib/pq"
)
"#,
        ),
        (
            "javascript",
            r#"import React from 'react';
import { useState, useEffect } from 'react';
const path = require('path');
"#,
        ),
        (
            "typescript",
            r#"import { Component } from '@angular/core';
import type { Config } from './config';
import * as fs from 'fs';
"#,
        ),
        (
            "java",
            r#"import com.example.UserService;
import static java.util.Collections.emptyList;
import java.util.*;
"#,
        ),
        (
            "c",
            r#"#include <stdio.h>
#include "mylib.h"
#include "../utils/helper.h"
"#,
        ),
        (
            "ruby",
            r#"require 'json'
require_relative './helper'
require_relative '../utils/parser'
"#,
        ),
    ];

    /// Recursively walk and print every node in the tree.
    fn print_tree(
        node: tree_sitter::Node,
        source: &[u8],
        lang: &str,
        depth: usize,
        field_name: Option<&str>,
    ) {
        let indent = "  ".repeat(depth);
        let kind = node.kind();
        let is_named = node.is_named();
        let start = node.start_byte();
        let end = node.end_byte();
        let text_raw = &source[start..end];
        // Truncate long text to keep output readable
        let text = std::str::from_utf8(text_raw).unwrap_or("<non-utf8>");
        let text_display = if text.len() > 80 {
            format!("{}...", &text[..77])
        } else {
            text.replace('\n', "\\n")
        };

        let field_str = match field_name {
            Some(f) => format!(" field:{}", f),
            None => String::new(),
        };
        let named_str = if is_named { "" } else { " [anon]" };

        println!(
            "{}{}  ({}-{}){}{} {:?}",
            indent, kind, start, end, field_str, named_str, text_display,
        );

        // Recurse into children, preserving field names
        let child_count = node.child_count();
        for i in 0..child_count {
            let child = node.child(i).unwrap();
            let child_field = node.field_name_for_child(i as u32);
            print_tree(child, source, lang, depth + 1, child_field);
        }
    }

    #[test]
    #[ignore]
    fn ast_elixir_multi_alias_dump() {
        let elixir_samples = &[
            ("elixir", r#"alias Acme.Shared.V1
alias Acme.Inventory.Domain.{Product, ProductNotFoundError, InsufficientStockError}
import Ecto.Query
use GenServer
require Logger
"#),
        ];
        let mut parser = Parser::new();
        for &(lang, source) in elixir_samples {
            println!("\n{}", "=".repeat(72));
            println!("[{}] Multi-alias AST dump", lang);
            println!("{}", "=".repeat(72));
            let config = match lang_registry::get(lang) {
                Some(c) => c,
                None => { println!("[{}] SKIPPED — plugin not installed", lang); continue; }
            };
            if let Err(e) = parser.set_language(&config.grammar) {
                println!("[{}] ERROR: {}", lang, e); continue;
            }
            let tree = match parser.parse(source.as_bytes(), None) {
                Some(t) => t,
                None => { println!("[{}] parse returned None", lang); continue; }
            };
            println!("[{}] Source:", lang);
            for (i, line) in source.lines().enumerate() {
                println!("  {:3}| {}", i + 1, line);
            }
            println!();
            println!("[{}] Full AST:", lang);
            print_tree(tree.root_node(), source.as_bytes(), lang, 0, None);
        }
    }

    #[test]
    #[ignore]
    fn ast_all_langs_dump() {
        let samples: &[(&str, &str)] = &[
            ("bash", "#!/bin/bash\nmy_func() {\n  echo hello\n}\nsource ./utils.sh\n. ./helper.sh\n"),
            ("gdscript", "const Utils = preload(\"res://scripts/utils.gd\")\nvar scene = load(\"res://scenes/main.tscn\")\nfunc greet(name):\n  print(name)\nclass_name Cat\n"),
            ("haskell", "module Main where\nimport Data.List\ndata Color = Red | Green | Blue\nclass Printable a where\n  display :: a -> String\nnewtype Name = Name String\ngreet name = name\n"),
            ("scala", "package com.example\nimport scala.collection.mutable\ndef greet(name: String): Unit = println(name)\nclass Cat\n"),
            ("zig", "const std = @import(\"std\");\nfn hello(name: []const u8) void {\n  _ = name;\n}\n"),
            ("html", "<html><head><link rel=\"stylesheet\" href=\"style.css\"></head><body></body></html>\n"),
            ("nim", "proc hello(name: string) =\n  echo name\ntype Cat = object\n  name: string\nimport strutils\nfrom os import joinPath\n"),
            ("julia", "function greet(name)\n  println(name)\nend\nstruct Point\n  x::Float64\nend\nimport LinearAlgebra\nusing Base: push!\n"),
            ("groovy", "def hello(name) {\n  println name\n}\nclass Cat {\n  String name\n}\nimport groovy.json.JsonSlurper\n"),
            ("powershell", "function Get-Hello {\n  param($Name)\n  Write-Host $Name\n}\nclass Animal {\n  [string]$Name\n}\n"),
            ("fsharp", "let greet name = printfn name\ntype Cat = { Name: string }\nmodule MyMod =\n  let x = 1\nopen System\n"),
            ("solidity", "pragma solidity ^0.8.0;\nimport \"./Ownable.sol\";\ncontract Token {\n  function transfer(address to) public {}\n}\n"),
            ("dart", "import 'dart:io';\nimport 'package:flutter/material.dart';\nvoid greet(String name) {\n  print(name);\n}\nclass Cat {\n  String name;\n}\n"),
            ("cobol", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO-WORLD.\n       PROCEDURE DIVISION.\n           DISPLAY \"Hello\".\n           STOP RUN.\n       COPY \"UTILS.CPY\".\n"),
            ("r", "library(ggplot2)\nhello <- function(x) { x }\n"),
            ("nix", "{ pkgs }:\nlet\n  utils = import ./utils.nix;\n  hello = name: \"hello\";\nin\n  hello pkgs\n"),
            ("objective-c", "#import <Foundation/Foundation.h>\n#import \"MyClass.h\"\n@interface Cat : NSObject\n@end\nvoid hello() {}\n"),
            ("ocaml", "let greet name = print_string name\nmodule M = struct end\ntype cat = { name: string }\nopen List\n"),
            ("perl", "package MyModule;\nuse strict;\nuse warnings;\nsub hello {\n  my $name = shift;\n  print $name;\n}\n1;\n"),
            ("erlang", "-module(mymod).\n-export([hello/1]).\n-import(lists, [map/2]).\nhello(Name) -> io:format(Name).\n"),
            ("kotlin", "package com.example\nimport kotlin.collections.List\nfun greet(name: String) {\n  println(name)\n}\nclass Cat(val name: String)\ninterface Animal\n"),
            ("protobuf", "syntax = \"proto3\";\nimport \"other.proto\";\nmessage Person {\n  string name = 1;\n}\nservice Greeter {\n  rpc Hello (Person) returns (Person);\n}\n"),
            ("svelte", "<script>\nimport { onMount } from 'svelte';\nfunction hello() {}\n</script>\n<h1>Hello</h1>\n"),
            ("vue", "<template><div>Hello</div></template>\n<script>\nimport axios from 'axios';\nexport default {\n  methods: { hello() {} }\n}\n</script>\n"),
        ];
        let mut parser = tree_sitter::Parser::new();
        for &(lang, source) in samples {
            println!("\n{}", "=".repeat(72));
            println!("[{}]", lang);
            println!("{}", "=".repeat(72));
            let config = match lang_registry::get(lang) {
                Some(c) => c,
                None => { println!("SKIPPED — not loaded"); continue; }
            };
            if let Err(e) = parser.set_language(&config.grammar) {
                println!("ERROR: {}", e); continue;
            }
            let tree = match parser.parse(source.as_bytes(), None) {
                Some(t) => t,
                None => { println!("parse returned None"); continue; }
            };
            print_tree(tree.root_node(), source.as_bytes(), lang, 0, None);
        }
    }

    #[test]
    #[ignore]
    fn ast_import_dump() {
        let mut parser = Parser::new();
        let mut found_any = false;

        for &(lang, source) in SAMPLES {
            println!("\n{}", "=".repeat(72));
            println!("[{}] Attempting to parse import statements", lang);
            println!("{}", "=".repeat(72));

            let config = match lang_registry::get(lang) {
                Some(c) => c,
                None => {
                    println!("[{}] SKIPPED — plugin not installed", lang);
                    continue;
                }
            };

            found_any = true;

            if let Err(e) = parser.set_language(&config.grammar) {
                println!("[{}] ERROR setting language: {}", lang, e);
                continue;
            }

            let tree = match parser.parse(source.as_bytes(), None) {
                Some(t) => t,
                None => {
                    println!("[{}] ERROR: parser.parse returned None", lang);
                    continue;
                }
            };

            println!("[{}] Source:", lang);
            for (i, line) in source.lines().enumerate() {
                println!("  {:3}| {}", i + 1, line);
            }
            println!();
            println!("[{}] Full AST:", lang);
            print_tree(tree.root_node(), source.as_bytes(), lang, 0, None);
            println!();
        }

        if !found_any {
            println!(
                "\nWARNING: No language plugins were loaded. \
                 Install plugins with `sentrux plugin add-standard`."
            );
        }
    }
}
