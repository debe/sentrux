//! Embedded plugin configs — auto-extracted at startup.
//! ALL 50 languages. Binary version = plugin version. Always in sync.
//!
//! Plugins without compiled grammars load config only (colors, extensions,
//! thresholds). Structural analysis activates when grammar .dylib is installed.

/// (name, plugin_toml_content, tags_scm_content)
pub const EMBEDDED_PLUGINS: &[(&str, &str, &str)] = &[
    ("bash",
r#"[plugin]
name = "bash"
display_name = "Bash"
version = "0.1.0"
extensions = ["sh", "bash"]
filenames = ["Makefile", "GNUmakefile", "Makefile.*", "Justfile"]
min_sentrux_version = "0.3.0"
color_rgb = [110, 160, 80]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-bash"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
hash_is_comment = true
is_executable = true

[semantics.project]

[semantics.complexity]
branch_nodes = ["if_statement", "case_statement", "for_statement", "c_style_for_statement", "while_statement"]
logic_nodes = []
logic_operators = []
nesting_nodes = ["if_statement", "case_statement", "for_statement", "c_style_for_statement", "while_statement"]
"#,
r#"; Bash structural queries

; Functions
(function_definition
  name: (word) @func.name) @func.def

; Commands (calls)
(command
  name: (command_name) @call.name) @call

; ---- Import appendix (custom) ----

; source ./file.sh / . ./file.sh (unquoted argument)
(command
  name: (command_name) @_cmd
  argument: (word) @import.module
  (#match? @_cmd "^(source|\\.)$")) @import

; source './file.sh' (quoted argument)
(command
  name: (command_name) @_cmd2
  argument: (raw_string) @import.module
  (#match? @_cmd2 "^(source|\\.)$")) @import

; source "/path/to/file.sh" (double-quoted argument)
(command
  name: (command_name) @_cmd3
  argument: (string) @import.module
  (#match? @_cmd3 "^(source|\\.)$")) @import
"#),
    ("c",
r#"[plugin]
name = "c"
display_name = "C"
version = "0.1.0"
extensions = ["c", "h"]
min_sentrux_version = "0.3.0"
color_rgb = [90, 95, 100]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-c"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["base_class_clause"]
is_executable = true
test_suffixes = ["_test.c", "_tests.c"]
test_dir_prefixes = ["test/", "tests/"]
test_dir_infixes = ["/test/", "/tests/"]

[semantics.project]
source_dirs = ["src", "include"]

[semantics.import_ast]
strategy = "field_read"
module_path_field = "path"
module_path_node_kinds = ["string_literal", "system_lib_string"]
string_content_kind = "string_content"
filter_system_includes = true
system_include_kind = "system_lib_string"

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement", "case_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch ", "&&", "||"]
cog_branch = ["if ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch "]
"#,
r#"; Official tree-sitter-c tags.scm (v0.23.4)

(struct_specifier name: (type_identifier) @name body:(_)) @definition.class

(declaration type: (union_specifier name: (type_identifier) @name)) @definition.class

(function_declarator declarator: (identifier) @name) @definition.function

(type_definition declarator: (type_identifier) @name) @definition.type

(enum_specifier name: (type_identifier) @name) @definition.type

; ---- Custom additions for imports/calls ----

; Pointer function declarations (official misses these)
(function_definition
  declarator: (pointer_declarator
    declarator: (function_declarator
      declarator: (identifier) @func.name))) @func.def

; Includes
(preproc_include
  path: (string_literal) @import.module) @import

(preproc_include
  path: (system_lib_string) @import.module) @import

; Calls — direct
(call_expression
  function: (identifier) @call.name) @call

; Calls — member  ptr->func() or obj.func()
(call_expression
  function: (field_expression
    field: (field_identifier) @call.name)) @call
"#),
    ("clojure",
r#"[plugin]
name = "clojure"
display_name = "Clojure"
version = "0.2.0"
extensions = ["clj", "cljs", "cljc"]
min_sentrux_version = "0.4.0"
color_rgb = [80, 145, 55]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/sogaiu/tree-sitter-clojure"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["deps.edn", "project.clj"]
ignored_dirs = ["target", ".cpcache", ".clj-kondo"]
source_dirs = ["src"]
"#,
r#"; Clojure tags.scm
; Clojure uses list forms: (defn name ...), (def name ...), (ns name ...)

(list_lit
  value: (sym_lit) @name
  (#match? @name "^(defn|defn-|defmacro|defmethod|defmulti)$")
  value: (sym_lit) @func.name) @definition.function

(list_lit
  value: (sym_lit) @name
  (#match? @name "^(defprotocol|defrecord|deftype|definterface)$")
  value: (sym_lit) @class.name) @definition.class

; ---- Import appendix ----
; UNTESTED: @import.module captures are best-effort without grammar validation

; ns form: (ns my.namespace (:require ...))
; The second sym_lit child after "ns" is the namespace (module path)
(list_lit
  value: (sym_lit) @_ns_kw
  (#eq? @_ns_kw "ns")
  value: (sym_lit) @import.module) @import

; require form: (require '[clojure.string :as str])
; Fallback: capture the whole require/use/import form
(list_lit
  value: (sym_lit) @_req_kw
  (#match? @_req_kw "^(require|use|import)$")) @import
"#),
    ("cpp",
r#"[plugin]
name = "cpp"
display_name = "C++"
version = "0.1.0"
extensions = ["cpp", "cc", "cxx", "hpp", "hh", "hxx"]
min_sentrux_version = "0.3.0"
color_rgb = [55, 90, 140]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-cpp"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["base_class_clause"]
is_executable = true
test_suffixes = ["_test.cpp", "_tests.cpp", "_test.cc"]
test_dir_prefixes = ["test/", "tests/"]
test_dir_infixes = ["/test/", "/tests/"]

[semantics.project]
source_dirs = ["src", "include"]

[semantics.import_ast]
strategy = "field_read"
module_path_field = "path"
module_path_node_kinds = ["string_literal", "system_lib_string"]
string_content_kind = "string_content"
filter_system_includes = true
system_include_kind = "system_lib_string"

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "for_range_loop", "while_statement", "do_statement", "switch_statement", "catch_clause", "case_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "for_range_loop", "while_statement", "do_statement", "switch_statement", "try_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch ", "&&", "||"]
cog_branch = ["if ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch "]
"#,
r#"; Official tree-sitter-cpp tags.scm (v0.23.4)

(struct_specifier name: (type_identifier) @name body:(_)) @definition.class

(declaration type: (union_specifier name: (type_identifier) @name)) @definition.class

(function_declarator declarator: (identifier) @name) @definition.function

(function_declarator declarator: (field_identifier) @name) @definition.function

(function_declarator declarator: (qualified_identifier scope: (namespace_identifier) @local.scope name: (identifier) @name)) @definition.method

(type_definition declarator: (type_identifier) @name) @definition.type

(enum_specifier name: (type_identifier) @name) @definition.type

(class_specifier name: (type_identifier) @name) @definition.class

; ---- Custom additions for imports/calls ----

; Pointer function declarations
(function_definition
  declarator: (pointer_declarator
    declarator: (function_declarator
      declarator: (identifier) @func.name))) @func.def

; Reference function declarations
(function_definition
  declarator: (reference_declarator
    (function_declarator
      declarator: (identifier) @func.name))) @func.def

; Includes
(preproc_include
  path: (string_literal) @import.module) @import

(preproc_include
  path: (system_lib_string) @import.module) @import

; Calls — direct
(call_expression
  function: (identifier) @call.name) @call

; Calls — member
(call_expression
  function: (field_expression
    field: (field_identifier) @call.name)) @call

; Calls — qualified  Foo::bar() or std::cout
(call_expression
  function: (qualified_identifier
    name: (identifier) @call.name)) @call

; Calls — new constructor  new Foo()
(new_expression
  type: (type_identifier) @call.name) @call
"#),
    ("crystal",
r#"[plugin]
name = "crystal"
display_name = "Crystal"
version = "0.2.0"
extensions = ["cr"]
min_sentrux_version = "0.4.0"
color_rgb = [0, 0, 0]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/crystal-lang-tools/tree-sitter-crystal"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
hash_is_comment = true
import_extractor = ""
base_class_extractor = "generic"

[semantics.project]
manifest_files = ["shard.yml"]
ignored_dirs = ["lib", ".shards"]
source_dirs = ["src"]

[semantics.resolver]
alias_file = "shard.yml"
alias_field = "name"
source_root = "src"

[semantics.complexity]
branch_nodes = ["if", "elsif", "for", "while", "unless", "until", "when", "rescue"]
logic_nodes = ["binary"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if", "for", "while", "unless", "case"]
"#,
r#"; Crystal tags.scm (Ruby-like syntax)

(method_def
  name: (identifier) @name) @definition.function

(class_def
  name: (constant) @name) @definition.class

(module_def
  name: (constant) @name) @definition.module

(struct_def
  name: (constant) @name) @definition.class

(call
  method: (identifier) @name) @reference.call

; ---- Import appendix ----

(require
  (string) @import.module) @import
"#),
    ("csharp",
r##"[plugin]
name = "csharp"
display_name = "C#"
version = "0.1.0"
extensions = ["cs"]
min_sentrux_version = "0.3.0"
color_rgb = [105, 60, 120]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-c-sharp"
ref = "master"
abi_version = 14
symbol_name = "c_sharp"

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["superclass", "super_interfaces", "type_list", "extends_type_clause", "class_type", "delegation_specifiers"]
abstract_keywords = ["abstract"]
is_executable = true
test_suffixes = ["Test.cs", "Tests.cs"]
test_dir_prefixes = ["test/", "tests/"]
test_dir_infixes = ["/test/", "/tests/"]
main_filenames = ["program.cs"]


[semantics.resolver]
alias_file = "*.csproj"
alias_field = "Project.PropertyGroup.AssemblyName"
source_root = "src"
[semantics.project]
manifest_files = ["*.csproj"]
source_dirs = ["src"]

[semantics.import_ast]
strategy = "scoped_path"
path_separator = "."
scoped_path_kinds = ["qualified_name", "identifier"]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "foreach_statement", "while_statement", "do_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "foreach_statement", "while_statement", "do_statement", "switch_statement", "try_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch ", "&&", "||"]
cog_branch = ["if ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch "]
"##,
r#"; Official tree-sitter-c-sharp tags.scm (v0.23.1)

(class_declaration name: (identifier) @name) @definition.class

(class_declaration (base_list (_) @name)) @reference.class

(interface_declaration name: (identifier) @name) @definition.interface

(interface_declaration (base_list (_) @name)) @reference.interface

(method_declaration name: (identifier) @name) @definition.method

(object_creation_expression type: (identifier) @name) @reference.class

(type_parameter_constraints_clause (identifier) @name) @reference.class

(type_parameter_constraint (type type: (identifier) @name)) @reference.class

(variable_declaration type: (identifier) @name) @reference.class

(invocation_expression function: (member_access_expression name: (identifier) @name)) @reference.send

(namespace_declaration name: (identifier) @name) @definition.module

; ---- Custom additions for structs/enums/constructors/imports/calls ----

; Structs
(struct_declaration
  name: (identifier) @class.name) @class.def

; Enums
(enum_declaration
  name: (identifier) @class.name) @class.def

; Constructor
(constructor_declaration
  name: (identifier) @func.name) @func.def

; Using directives
(using_directive) @import

; Calls — direct
(invocation_expression
  function: (identifier) @call.name) @call
"#),
    ("css",
r#"[plugin]
name = "css"
display_name = "CSS"
version = "0.1.0"
extensions = ["css"]
min_sentrux_version = "0.3.0"
color_rgb = [85, 70, 120]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-css"
ref = "master"
abi_version = 14

[queries]
capabilities = ["imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = false

[semantics.project]
"#,
r#"; CSS structural queries
; CSS has no functions/classes in the traditional sense

; ---- Import appendix ----

; @import "file.css" or @import url("file.css")
; Capture the string value as import.module
(import_statement
  [(string_value) (call_expression)] @import.module) @import
"#),
    ("dart",
r#"[plugin]
name = "dart"
display_name = "Dart"
version = "0.2.0"
extensions = ["dart"]
min_sentrux_version = "0.4.0"
color_rgb = [100, 180, 220]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/UserNobody14/tree-sitter-dart"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""
base_class_extractor = "generic"

[semantics.project]
manifest_files = ["pubspec.yaml"]
ignored_dirs = [".dart_tool", "build", ".pub-cache"]
source_dirs = ["lib"]

[semantics.resolver]
alias_file = "pubspec.yaml"
alias_field = "name"
source_root = "lib"

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement"]
"#,
r#"(function_signature) @definition.function
(method_signature) @definition.function
(class_definition) @definition.class
(enum_declaration) @definition.class
(mixin_declaration) @definition.class
(library_import) @import
(library_export) @import
"#),
    ("dockerfile",
r#"[plugin]
name = "dockerfile"
display_name = "Dockerfile"
version = "0.2.0"
extensions = ["dockerfile"]
filenames = ["Dockerfile", "Dockerfile.*"]
min_sentrux_version = "0.4.0"
color_rgb = [60, 80, 90]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/camdencheek/tree-sitter-dockerfile"
ref = "main"
abi_version = 14

[queries]
capabilities = []

[checksums]

[semantics]
is_executable = false
"#,
r#"; Dockerfile — display-only language, no structural analysis
"#),
    ("elixir",
r#"[plugin]
name = "elixir"
display_name = "Elixir"
version = "0.1.0"
extensions = ["ex", "exs"]
min_sentrux_version = "0.3.0"
color_rgb = [100, 75, 120]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/elixir-lang/tree-sitter-elixir"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
base_class_extractor = "generic"
is_executable = true
test_suffixes = ["_test.exs"]
test_dir_prefixes = ["test/"]
test_dir_infixes = ["/test/"]


[semantics.resolver]
alias_file = "mix.exs"
alias_field = "app:"
source_root = "lib"
[semantics.project]
manifest_files = ["mix.exs"]
source_dirs = ["lib"]

[semantics.import_ast]
strategy = "field_read"
module_path_field = ""
module_path_node_kinds = ["alias"]
module_name_transform = "pascal_to_snake"
recursive_search = true

[semantics.complexity]
branch_nodes = []
logic_nodes = []
nesting_nodes = []
"#,
r#"; Official tree-sitter-elixir tags.scm (v0.3.5)

; modules and protocols
(call
  target: (identifier) @_kw
  (arguments (alias) @name)
  (#any-of? @_kw "defmodule" "defprotocol")) @definition.module

; functions/macros
(call
  target: (identifier) @_kw
  (arguments
    [
      (identifier) @name
      (call target: (identifier) @name)
      (binary_operator
        left: (call target: (identifier) @name)
        operator: "when")
    ])
  (#any-of? @_kw "def" "defp" "defdelegate" "defguard" "defguardp" "defmacro" "defmacrop" "defn" "defnp")) @definition.function

; ignore kernel/special-forms
(call
  target: (identifier) @_kw
  (#any-of? @_kw "def" "defp" "defdelegate" "defguard" "defguardp" "defmacro" "defmacrop" "defn" "defnp" "defmodule" "defprotocol" "defimpl" "defstruct" "defexception" "defoverridable" "alias" "case" "cond" "else" "for" "if" "import" "quote" "raise" "receive" "require" "reraise" "super" "throw" "try" "unless" "unquote" "unquote_splicing" "use" "with"))

; function calls
(call
  target: [
   (identifier) @name
   (dot
     right: (identifier) @name)
  ]) @reference.call

; pipe into function call
(binary_operator
  operator: "|>"
  right: (identifier) @name) @reference.call

; ---- Import appendix (custom) ----
; alias/import/use/require with alias argument (PascalCase module)
(call
  target: (identifier) @_import_kw
  (arguments (alias) @import.module)
  (#any-of? @_import_kw "alias" "import" "use" "require")) @import

; alias/import/use/require without alias (fallback — captures whole call)
(call
  target: (identifier) @_import_kw2
  (#any-of? @_import_kw2 "alias" "import" "use" "require")) @import
"#),
    ("erlang",
r#"[plugin]
name = "erlang"
display_name = "Erlang"
version = "0.2.0"
extensions = ["erl", "hrl"]
min_sentrux_version = "0.4.0"
color_rgb = [160, 50, 50]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/WhatsApp/tree-sitter-erlang"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""

[semantics.project]
source_dirs = ["src"]
ignored_dirs = ["_build", "deps"]
manifest_files = ["rebar.config"]

[semantics.complexity]
branch_nodes = ["if_expr", "case_expr", "receive_expr", "try_expr", "catch_clause"]
logic_nodes = []
logic_operators = ["andalso", "orelse"]
nesting_nodes = ["if_expr", "case_expr", "receive_expr", "try_expr"]
"#,
r#"; Erlang tags.scm

(fun_decl
  (function_clause
    name: (atom) @name)) @definition.function

(module_attribute
  name: (atom) @name) @definition.module

(import_attribute) @import
"#),
    ("fsharp",
r##"[plugin]
name = "fsharp"
display_name = "F#"
version = "0.2.0"
extensions = ["fs", "fsx", "fsi"]
min_sentrux_version = "0.4.0"
color_rgb = [55, 120, 185]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/ionide/tree-sitter-fsharp"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["*.fsproj"]
ignored_dirs = ["bin", "obj", ".fable"]
source_dirs = ["src"]

[semantics.resolver]
alias_file = "*.fsproj"
alias_field = "Project.PropertyGroup.AssemblyName"

[semantics.complexity]
branch_nodes = ["if_expression", "match_expression", "for_expression", "while_expression", "try_expression"]
logic_nodes = ["infix_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_expression", "match_expression", "for_expression", "while_expression", "try_expression"]
"##,
r#"(function_or_value_defn) @definition.function
(type_definition) @definition.class
(module_defn) @definition.module
(import_decl) @import
"#),
    ("gdscript",
r#"[plugin]
name = "gdscript"
display_name = "GDScript (Godot)"
version = "0.1.0"
extensions = ["gd"]
min_sentrux_version = "0.1.3"
color_rgb = [80, 85, 90]

[plugin.metadata]
author = "sentrux community"
homepage = "https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript"
license = "MIT"
description = "GDScript support for Godot game engine projects"

[grammar]
source = "https://github.com/PrestonKnopp/tree-sitter-gdscript"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes"]

[checksums]
# Populated by CI after building

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = true

[semantics.project]
"#,
r#";; GDScript structural queries for sentrux

;; Function definitions
(function_definition
  name: (name) @func.name) @func.def

;; Class definitions
(class_definition
  name: (name) @class.name) @class.def

;; All calls — captured as reference.call for call graph
(call) @reference.call

;; ---- Import appendix (custom) ----

;; preload("res://path") / load("res://path")
;; Capture string arguments inside calls as import.module
(call
  (arguments
    (string) @import.module)) @import
"#),
    ("glsl",
r#"[plugin]
name = "glsl"
display_name = "GLSL"
version = "0.2.0"
extensions = ["glsl", "vert", "frag", "comp"]
min_sentrux_version = "0.4.0"
color_rgb = [90, 200, 90]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-glsl"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = false
import_extractor = ""

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement", "case_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "do_statement"]
"#,
r#"; GLSL tags.scm

(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name)) @definition.function

(struct_specifier
  name: (type_identifier) @name) @definition.class

; ---- Import appendix ----

(preproc_include
  path: (_) @import.module) @import
"#),
    ("go",
r#"[plugin]
name = "go"
display_name = "Go"
version = "0.1.0"
extensions = ["go"]
min_sentrux_version = "0.3.0"
color_rgb = [55, 140, 165]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-go"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = true
main_filenames = ["server.go", "app.go"]

[semantics.project]
manifest_files = ["go.mod"]
source_dirs = ["cmd", "pkg", "internal"]
directory_is_package = true

[semantics.resolver]
module_prefix_file = "go.mod"
module_prefix_directive = "module"
workspace_file = "go.work"
workspace_format = "go_work"
workspace_members_field = "use"
workspace_package_name_field = ""
workspace_entry_point = ""

[semantics.import_ast]
strategy = "field_read"
module_path_field = "path"
module_path_node_kinds = ["interpreted_string_literal"]
string_content_kind = "interpreted_string_literal_content"
child_import_kind = "import_spec"

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "expression_switch_statement", "type_switch_statement", "select_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "expression_switch_statement", "type_switch_statement", "select_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "else if", "for ", "switch ", "select ", "case ", "&&", "||"]
cog_branch = ["if ", "else if", "for ", "switch ", "select ", "case "]
"#,
r#"; Official tree-sitter-go tags.scm (v0.23.4)

(
  (comment)* @doc
  .
  (function_declaration
    name: (identifier) @name) @definition.function
  (#strip! @doc "^//\\s*")
  (#set-adjacent! @doc @definition.function)
)

(
  (comment)* @doc
  .
  (method_declaration
    name: (field_identifier) @name) @definition.method
  (#strip! @doc "^//\\s*")
  (#set-adjacent! @doc @definition.method)
)

(call_expression
  function: [
    (identifier) @name
    (parenthesized_expression (identifier) @name)
    (selector_expression field: (field_identifier) @name)
    (parenthesized_expression (selector_expression field: (field_identifier) @name))
  ]) @reference.call

(type_spec
  name: (type_identifier) @name) @definition.type

; ---- Import appendix (custom) ----

(import_declaration) @import
"#),
    ("groovy",
r#"[plugin]
name = "groovy"
display_name = "Groovy"
version = "0.2.0"
extensions = ["groovy", "gvy"]
min_sentrux_version = "0.4.0"
color_rgb = [70, 152, 182]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/murtaza64/tree-sitter-groovy"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""
base_class_extractor = "generic"

[semantics.project]
manifest_files = ["build.gradle"]
ignored_dirs = ["build", ".gradle"]
source_dirs = ["src"]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "switch_statement"]
"#,
r#"(function_definition) @definition.function
(function_declaration) @definition.function
(class_definition) @definition.class
(groovy_import) @import
"#),
    ("haskell",
r#"[plugin]
name = "haskell"
display_name = "Haskell"
version = "0.1.0"
extensions = ["hs"]
min_sentrux_version = "0.3.0"
color_rgb = [90, 80, 125]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-haskell"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = true

[semantics.project]

[semantics.complexity]
branch_nodes = ["case", "lambda", "list_comprehension"]
logic_nodes = []
logic_operators = []
nesting_nodes = ["case", "lambda", "do", "list_comprehension"]
"#,
r#"; Haskell structural queries (hand-written, no official tags.scm)

; Function bindings
(function
  name: (variable) @func.name) @func.def

; Type class declarations
(class
  name: (name) @class.name) @class.def

; Data type declarations
(data_type
  name: (name) @class.name) @class.def

; Newtype declarations
(newtype
  name: (name) @class.name) @class.def

; Import declarations
(import
  module: (module) @import.module) @import
"#),
    ("hcl",
r#"[plugin]
name = "hcl"
display_name = "HCL"
version = "0.2.0"
extensions = ["tf", "hcl"]
min_sentrux_version = "0.4.0"
color_rgb = [100, 70, 180]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-hcl"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions"]

[checksums]

[semantics]
is_executable = false
import_extractor = ""

[semantics.project]
ignored_dirs = [".terraform"]
"#,
r#"; HCL / Terraform tags.scm

(block
  (identifier) @block.type
  (string_lit) @name) @definition.function

(attribute
  (identifier) @name) @definition.constant
"#),
    ("html",
r#"[plugin]
name = "html"
display_name = "HTML"
version = "0.1.0"
extensions = ["html", "htm"]
min_sentrux_version = "0.3.0"

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-html"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = false

[semantics.project]
"#,
r#"; HTML structural queries

; Capture <script> and <style> inline blocks as class-like structures
(script_element) @definition.class
(style_element) @definition.class

; ---- Import appendix (custom) ----

; <script src="./app.js"> — only src attribute
(script_element
  (start_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value) @import.module)
    (#eq? @_attr "src"))) @import

; <link href="./style.css"> — only href attribute on self-closing tags
(self_closing_tag
  (tag_name) @_tag
  (attribute
    (attribute_name) @_attr
    (quoted_attribute_value) @import.module)
  (#eq? @_tag "link")
  (#eq? @_attr "href")) @import

; <img src="./logo.png">, <source src="...">, etc.
(element
  (start_tag
    (tag_name) @_tag
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value) @import.module)
    (#any-of? @_tag "img" "source" "video" "audio" "iframe" "embed")
    (#eq? @_attr "src"))) @import
"#),
    ("java",
r#"[plugin]
name = "java"
display_name = "Java"
version = "0.1.0"
extensions = ["java"]
min_sentrux_version = "0.3.0"
color_rgb = [150, 110, 55]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-java"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["superclass", "super_interfaces", "type_list", "extends_type_clause", "class_type", "delegation_specifiers"]
abstract_keywords = ["abstract"]
is_executable = true
test_suffixes = ["Test.java", "Tests.java"]
test_dir_prefixes = ["test/", "tests/"]
test_dir_infixes = ["/test/", "/tests/"]
main_filenames = ["app.java"]


[semantics.resolver]
alias_file = "pom.xml"
alias_field = "project.artifactId"
alias_entry_point = "src/main/java"
source_root = "src/main/java"
[semantics.project]
manifest_files = ["pom.xml", "build.gradle", "build.gradle.kts"]
source_dirs = ["src"]

[semantics.import_ast]
strategy = "scoped_path"
path_separator = "."
scoped_path_kinds = ["scoped_identifier"]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "enhanced_for_statement", "while_statement", "do_statement", "switch_expression", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "enhanced_for_statement", "while_statement", "do_statement", "switch_expression", "try_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch ", "&&", "||"]
cog_branch = ["if ", "if(", "else if", "for ", "for(", "while ", "while(", "switch ", "case ", "catch "]
"#,
r#"; Official tree-sitter-java tags.scm (v0.23.5)

(class_declaration
  name: (identifier) @name) @definition.class

(method_declaration
  name: (identifier) @name) @definition.method

(method_invocation
  name: (identifier) @name
  arguments: (argument_list) @reference.call)

(interface_declaration
  name: (identifier) @name) @definition.interface

(type_list
  (type_identifier) @name) @reference.implementation

(object_creation_expression
  type: (type_identifier) @name) @reference.class

(superclass (type_identifier) @name) @reference.class

; ---- Import appendix + custom additions ----

(import_declaration) @import

; Constructors (not in official tags.scm)
(constructor_declaration
  name: (identifier) @func.name) @func.def
"#),
    ("javascript",
r#"[plugin]
name = "javascript"
display_name = "JavaScript"
version = "0.1.0"
extensions = ["js", "mjs", "cjs", "jsx"]
min_sentrux_version = "0.3.0"
color_rgb = [175, 165, 85]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-javascript"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["class_heritage", "extends_clause"]
package_index_files = ["index.js", "index.mjs", "index.cjs", "index.jsx"]
is_executable = true
test_suffixes = [".test.js", ".test.jsx", ".spec.js", ".spec.jsx"]
test_dir_prefixes = ["__tests__/", "test/", "tests/"]
test_dir_infixes = ["/__tests__/", "/test/", "/tests/"]
main_filenames = ["app.js", "server.js"]

[semantics.project]
manifest_files = ["package.json"]
ignored_dirs = ["node_modules", "dist", "build", ".next", "coverage"]
source_dirs = ["src", "lib", "packages"]

[semantics.resolver]
alias_file = "package.json"
alias_field = "name"
alias_entry_point = "src/index.js"
path_alias_file = "tsconfig.json"
path_alias_field = "compilerOptions.paths"
path_alias_base_url = "compilerOptions.baseUrl"
resolve_extensions = [".js", ".jsx", ".mjs", ".cjs", ".json"]
source_root = "src"
workspace_file = "package.json"
workspace_format = "json"
workspace_members_field = "workspaces"
workspace_package_name_field = "name"
workspace_entry_point = "src/index.js"

[semantics.import_ast]
strategy = "field_read"
module_path_field = "source"
module_path_node_kinds = ["string"]
string_content_kind = "string_fragment"

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "for_in_statement", "while_statement", "do_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "for_in_statement", "while_statement", "do_statement", "switch_statement", "try_statement"]
"#,
r#"; Official tree-sitter-javascript tags.scm (v0.23.1)

(
  (comment)* @doc
  .
  (method_definition
    name: (property_identifier) @name) @definition.method
  (#not-eq? @name "constructor")
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.method)
)

(
  (comment)* @doc
  .
  [
    (class
      name: (_) @name)
    (class_declaration
      name: (_) @name)
  ] @definition.class
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.class)
)

(
  (comment)* @doc
  .
  [
    (function_expression
      name: (identifier) @name)
    (function_declaration
      name: (identifier) @name)
    (generator_function
      name: (identifier) @name)
    (generator_function_declaration
      name: (identifier) @name)
  ] @definition.function
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.function)
)

(
  (comment)* @doc
  .
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name
      value: [(arrow_function) (function_expression)]) @definition.function)
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.function)
)

(
  (comment)* @doc
  .
  (variable_declaration
    (variable_declarator
      name: (identifier) @name
      value: [(arrow_function) (function_expression)]) @definition.function)
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.function)
)

(assignment_expression
  left: [
    (identifier) @name
    (member_expression
      property: (property_identifier) @name)
  ]
  right: [(arrow_function) (function_expression)]
) @definition.function

(pair
  key: (property_identifier) @name
  value: [(arrow_function) (function_expression)]) @definition.function

(
  (call_expression
    function: (identifier) @name) @reference.call
  (#not-match? @name "^(require)$")
)

(call_expression
  function: (member_expression
    property: (property_identifier) @name)
  arguments: (_) @reference.call)

(new_expression
  constructor: (_) @name) @reference.class

(export_statement value: (assignment_expression left: (identifier) @name right: ([
 (number)
 (string)
 (identifier)
 (undefined)
 (null)
 (new_expression)
 (binary_expression)
 (call_expression)
]))) @definition.constant

; ---- Import appendix (custom) ----

(import_statement
  source: (string) @import.module) @import
"#),
    ("json",
r#"[plugin]
name = "json"
display_name = "JSON"
version = "0.1.0"
extensions = ["json", "jsonc"]
color_rgb = [60, 65, 70]

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-json"
ref = "master"
abi_version = 14

[queries]
capabilities = []

[checksums]

[semantics]
is_executable = false
"#,
r#"; JSON — display only, no structural analysis
"#),
    ("julia",
r#"[plugin]
name = "julia"
display_name = "Julia"
version = "0.2.0"
extensions = ["jl"]
min_sentrux_version = "0.4.0"
color_rgb = [149, 88, 178]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-julia"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["Project.toml"]
ignored_dirs = [".julia", "deps"]
source_dirs = ["src"]

[semantics.resolver]
alias_file = "Project.toml"
alias_field = "name"
source_root = "src"

[semantics.complexity]
branch_nodes = ["if_statement", "elseif_clause", "for_statement", "while_statement", "try_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "try_statement"]
"#,
r#"(function_definition) @definition.function
(macro_definition) @definition.function
(struct_definition) @definition.class
(abstract_definition) @definition.class
(module_definition) @definition.module
(import_statement) @import
(using_statement) @import
"#),
    ("kotlin",
r#"[plugin]
name = "kotlin"
display_name = "Kotlin"
version = "0.2.0"
extensions = ["kt", "kts"]
min_sentrux_version = "0.4.0"
color_rgb = [135, 105, 190]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/fwcd/tree-sitter-kotlin"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
is_executable = true
import_extractor = ""
base_class_extractor = "generic"

[semantics.project]
manifest_files = ["build.gradle.kts", "build.gradle"]
ignored_dirs = ["build", ".gradle", ".idea"]
source_dirs = ["src"]

[semantics.complexity]
branch_nodes = ["if_expression", "when_expression", "for_statement", "while_statement", "do_while_statement", "catch_block"]
logic_nodes = ["conjunction_expression", "disjunction_expression"]
logic_operators = []
nesting_nodes = ["if_expression", "for_statement", "while_statement", "do_while_statement", "when_expression"]
"#,
r#"; Kotlin tags.scm

(function_declaration
  (simple_identifier) @name) @definition.function

(class_declaration
  (type_identifier) @name) @definition.class

(object_declaration
  (type_identifier) @name) @definition.class

(import_header
  (identifier) @import.module) @import
"#),
    ("lua",
r#"[plugin]
name = "lua"
display_name = "Lua"
version = "0.1.0"
extensions = ["lua"]
min_sentrux_version = "0.3.0"
color_rgb = [50, 55, 120]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-lua"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = true

[semantics.project]

[semantics.complexity]
branch_nodes = ["if_statement", "while_statement", "repeat_statement", "for_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["and", "or"]
nesting_nodes = ["if_statement", "while_statement", "repeat_statement", "for_statement"]
"#,
r#"; Official tree-sitter-lua tags.scm (v0.5.0)

(function_declaration
  name: [
    (identifier) @name
    (dot_index_expression
      field: (identifier) @name)
  ]) @definition.function

(function_declaration
  name: (method_index_expression
    method: (identifier) @name)) @definition.method

(assignment_statement
  (variable_list
    .
    name: [
      (identifier) @name
      (dot_index_expression
        field: (identifier) @name)
    ])
  (expression_list
    .
    value: (function_definition))) @definition.function

(table_constructor
  (field
    name: (identifier) @name
    value: (function_definition))) @definition.function

(function_call
  name: [
    (identifier) @name
    (dot_index_expression
      field: (identifier) @name)
    (method_index_expression
      method: (identifier) @name)
  ]) @reference.call

; ---- Import appendix (custom) ----

; require("module")
(function_call
  name: (identifier) @_fn
  arguments: (arguments
    (string) @import.module)
  (#eq? @_fn "require")) @import
"#),
    ("markdown",
r#"[plugin]
name = "markdown"
display_name = "Markdown"
version = "0.2.0"
extensions = ["md", "mdx"]
min_sentrux_version = "0.4.0"
color_rgb = [50, 70, 135]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-markdown"
ref = "split_parser"
abi_version = 14

[queries]
capabilities = []

[checksums]

[semantics]
is_executable = false
"#,
r#"; Markdown — display-only language, no structural analysis
"#),
    ("nim",
r#"[plugin]
name = "nim"
display_name = "Nim"
version = "0.2.0"
extensions = ["nim", "nims"]
min_sentrux_version = "0.4.0"
color_rgb = [255, 225, 95]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/aMOPel/tree-sitter-nim"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
hash_is_comment = true
import_extractor = ""

[semantics.project]
manifest_files = ["*.nimble"]
ignored_dirs = ["nimcache"]

[semantics.complexity]
branch_nodes = ["if_statement", "elif_branch", "for_statement", "while_statement", "case_statement", "try_statement", "except_branch"]
logic_nodes = ["infix_expression"]
logic_operators = ["and", "or"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "case_statement", "try_statement"]
"#,
r#"(routine) @definition.function
(typeDef) @definition.class
(objectDecl) @definition.class
(enumDecl) @definition.class
(importStmt) @import
(importExceptStmt) @import
(fromStmt) @import
(includeStmt) @import
"#),
    ("nix",
r#"[plugin]
name = "nix"
display_name = "Nix"
version = "0.2.0"
extensions = ["nix"]
min_sentrux_version = "0.4.0"
color_rgb = [126, 186, 228]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/nix-community/tree-sitter-nix"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
is_executable = false
hash_is_comment = true
import_extractor = ""
"#,
r#"(binding) @definition.function
(inherit) @definition.function
(with_expression) @import
"#),
    ("objective-c",
r#"[plugin]
name = "objective-c"
display_name = "Objective-C"
version = "0.2.0"
extensions = ["m", "h"]
min_sentrux_version = "0.4.0"
color_rgb = [65, 105, 225]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-objc"
ref = "master"
abi_version = 14
symbol_name = "objc"

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""
base_class_extractor = "generic"

[semantics.project]
source_dirs = ["src"]
ignored_dirs = ["build", "DerivedData"]
implicit_module = true
[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "do_statement", "switch_statement"]
"#,
r#"; Objective-C tags.scm

(function_definition) @definition.function
(method_declaration) @definition.function
(class_interface) @definition.class
(protocol_declaration) @definition.interface

; #import and #include
(preproc_include) @import
"#),
    ("ocaml",
r#"[plugin]
name = "ocaml"
display_name = "OCaml"
version = "0.2.0"
extensions = ["ml", "mli"]
min_sentrux_version = "0.4.0"
color_rgb = [180, 110, 45]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-ocaml"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["dune-project"]
ignored_dirs = ["_build", "_opam"]
source_dirs = ["lib", "bin"]

[semantics.complexity]
branch_nodes = ["if_expression", "match_expression", "for_expression", "while_expression", "try_expression"]
logic_nodes = ["infix_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_expression", "match_expression", "for_expression", "while_expression", "try_expression"]
"#,
r#"; OCaml tags.scm

(value_definition
  (let_binding
    pattern: (value_name) @name)) @definition.function

(type_definition
  (type_binding
    name: (type_constructor) @name)) @definition.class

(module_definition
  (module_binding
    (module_name) @name)) @definition.module

; ---- Import appendix ----

(open_module
  (module_path) @import.module) @import
"#),
    ("perl",
r#"[plugin]
name = "perl"
display_name = "Perl"
version = "0.2.0"
extensions = ["pl", "pm"]
min_sentrux_version = "0.4.0"
color_rgb = [85, 85, 120]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-perl/tree-sitter-perl"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
hash_is_comment = true
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["Makefile.PL", "Build.PL", "cpanfile"]
ignored_dirs = ["blib", ".build"]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "unless_statement", "until_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||", "and", "or"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "unless_statement"]
"#,
r#"; Perl tags.scm

(subroutine_declaration_statement
  name: (bareword) @name) @definition.function

(method_declaration_statement
  name: (bareword) @name) @definition.function

(package_statement
  name: (package) @name) @definition.class

(use_statement
  module: (package) @import.module) @import

(require_expression) @import
"#),
    ("php",
r#"[plugin]
name = "php"
display_name = "PHP"
version = "0.1.0"
extensions = ["php"]
min_sentrux_version = "0.3.0"
color_rgb = [105, 110, 150]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-php"
ref = "master"
abi_version = 14
symbol_name = "php_only"

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["base_clause", "class_interface_clause"]
is_executable = true
test_suffixes = ["Test.php"]
test_dir_prefixes = ["tests/", "test/"]
test_dir_infixes = ["/tests/", "/test/"]
main_filenames = ["index.php", "app.php", "server.php"]


[semantics.resolver]
alias_file = "composer.json"
alias_field = "name"
alias_entry_point = "src/index.php"
source_root = "src"
[semantics.project]
manifest_files = ["composer.json"]
ignored_dirs = ["vendor"]
source_dirs = ["src"]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "foreach_statement", "while_statement", "do_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||", "and", "or"]
nesting_nodes = ["if_statement", "for_statement", "foreach_statement", "while_statement", "do_statement", "switch_statement", "try_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "if(", "elseif ", "for ", "for(", "foreach ", "while ", "while(", "switch ", "case ", "catch ", "&&", "||"]
cog_branch = ["if ", "if(", "elseif ", "for ", "for(", "foreach ", "while ", "while(", "switch ", "case ", "catch "]
"#,
r#"; Official tree-sitter-php tags.scm (v0.23.11)

(namespace_definition
  name: (namespace_name) @name) @definition.module

(interface_declaration
  name: (name) @name) @definition.interface

(trait_declaration
  name: (name) @name) @definition.interface

(class_declaration
  name: (name) @name) @definition.class

(class_interface_clause [(name) (qualified_name)] @name) @reference.implementation

(property_declaration
  (property_element (variable_name (name) @name))) @definition.field

(function_definition
  name: (name) @name) @definition.function

(method_declaration
  name: (name) @name) @definition.function

(object_creation_expression
  [
    (qualified_name (name) @name)
    (variable_name (name)) @name
  ]) @reference.class

(function_call_expression
  function: [
    (qualified_name (name) @name)
    (variable_name (name)) @name
  ]) @reference.call

(scoped_call_expression
  name: (name) @name) @reference.call

(member_call_expression
  name: (name) @name) @reference.call

; ---- Import appendix (custom) ----

; use App\Models\User;
(namespace_use_declaration
  (namespace_use_clause
    [(qualified_name) (name)] @import.module)) @import

; require_once 'file.php' / include 'file.php'
(include_expression
  (string (string_content) @import.module)) @import
"#),
    ("powershell",
r#"[plugin]
name = "powershell"
display_name = "PowerShell"
version = "0.2.0"
extensions = ["ps1", "psm1", "psd1"]
min_sentrux_version = "0.4.0"
color_rgb = [1, 36, 86]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/airbus-cert/tree-sitter-powershell"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes"]

[checksums]

[semantics]
hash_is_comment = true
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["*.psd1"]
ignored_dirs = ["bin", "obj"]

[semantics.complexity]
branch_nodes = ["if_statement", "elseif_clause", "for_statement", "foreach_statement", "while_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["-and", "-or"]
nesting_nodes = ["if_statement", "for_statement", "foreach_statement", "while_statement", "switch_statement"]
"#,
r#"(function_statement) @definition.function
(class_statement) @definition.class
(enum_statement) @definition.class
"#),
    ("protobuf",
r#"[plugin]
name = "protobuf"
display_name = "Protocol Buffers"
version = "0.2.0"
extensions = ["proto"]
min_sentrux_version = "0.4.0"
color_rgb = [60, 60, 60]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/90-008/tree-sitter-protobuf"
ref = "main"
abi_version = 14

[queries]
capabilities = ["classes", "imports"]

[checksums]

[semantics]
is_executable = false
import_extractor = ""
"#,
r#"; Protocol Buffers tags.scm

(message
  (messageName) @name) @definition.class

(enum
  (enumName) @name) @definition.class

(service
  (serviceName) @name) @definition.class

(rpc
  (rpcName) @name) @definition.function

(import
  (strLit) @import.module) @import
"#),
    ("python",
r#"[plugin]
name = "python"
display_name = "Python"
version = "0.2.0"
extensions = ["py"]
min_sentrux_version = "0.4.0"
color_rgb = [65, 105, 145]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-python"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["argument_list", "superclasses"]
hash_is_comment = true
has_triple_quote_strings = true
package_index_files = ["__init__.py"]
abstract_base_classes = ["Protocol", "ABC", "ABCMeta"]
is_executable = true
test_suffixes = ["_test.py"]
test_prefixes = ["test_"]
test_dir_prefixes = ["tests/", "test/"]
test_dir_infixes = ["/tests/", "/test/"]
main_filenames = ["app.py", "server.py"]
entry_point_patterns = ["__main__"]


[semantics.resolver]
alias_file = "pyproject.toml"
alias_field = "project.name"
alias_entry_point = "__init__.py"
source_root = "src"
[semantics.project]
manifest_files = ["pyproject.toml", "setup.py"]
ignored_dirs = ["__pycache__", ".mypy_cache", ".pytest_cache", "venv", ".venv", ".tox", ".eggs", "site-packages"]
mod_declaration_files = ["__init__.py"]

[semantics.import_ast]
strategy = "field_read"
module_path_field = "module_name"
module_path_node_kinds = ["dotted_name", "relative_import"]
relative_import_kind = "relative_import"
import_prefix_kind = "import_prefix"

[semantics.complexity]
branch_nodes = ["if_statement", "elif_clause", "for_statement", "while_statement", "except_clause", "with_statement"]
logic_nodes = ["boolean_operator"]
logic_operators = []
nesting_nodes = ["if_statement", "for_statement", "while_statement", "with_statement", "try_statement"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "elif ", "for ", "while ", "except ", " and ", " or "]
cog_branch = ["if ", "elif ", "for ", "while ", "except ", "else:"]
cog_nesting = ["if ", "elif ", "for ", "while "]
"#,
r#"; Official tree-sitter-python tags.scm (v0.23.6)

(module (expression_statement (assignment left: (identifier) @name) @definition.constant))

(class_definition
  name: (identifier) @name) @definition.class

(function_definition
  name: (identifier) @name) @definition.function

(call
  function: [
      (identifier) @name
      (attribute
        attribute: (identifier) @name)
  ]) @reference.call

; ---- Entry point: if __name__ == "__main__" ----
(if_statement
  condition: (comparison_operator) @entry)

; ---- Import appendix (custom) ----

(import_from_statement
  module_name: (dotted_name) @import.module) @import

(import_statement) @import
"#),
    ("r",
r#"[plugin]
name = "r"
display_name = "R"
version = "0.1.0"
extensions = ["r", "R"]
min_sentrux_version = "0.3.0"
color_rgb = [50, 120, 175]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/r-lib/tree-sitter-r"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
hash_is_comment = true
is_executable = true

[semantics.project]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement", "repeat_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||", "&", "|"]
nesting_nodes = ["if_statement", "for_statement", "while_statement", "repeat_statement"]
"#,
r#"; Official tree-sitter-r tags.scm (v1.2.0)

(binary_operator
    lhs: (identifier) @name
    operator: "<-"
    rhs: (function_definition)
) @definition.function

(binary_operator
    lhs: (identifier) @name
    operator: "="
    rhs: (function_definition)
) @definition.function

(binary_operator
    lhs: (string) @name
    operator: "<-"
    rhs: (function_definition)
) @definition.function

(binary_operator
    lhs: (string) @name
    operator: "="
    rhs: (function_definition)
) @definition.function

(call
    function: (identifier) @name
) @reference.call

(call
    function: (namespace_operator
        rhs: (identifier) @name
    )
) @reference.call

; ---- Import appendix (custom) ----

; library("package") / require("package") / source("file.R")
(call
    function: (identifier) @_fn
    arguments: (arguments
        (string) @import.module)
    (#match? @_fn "^(library|require|source)$")) @import
"#),
    ("ruby",
r#"[plugin]
name = "ruby"
display_name = "Ruby"
version = "0.1.0"
extensions = ["rb"]
filenames = ["Rakefile", "Gemfile", "Guardfile", "Vagrantfile"]
min_sentrux_version = "0.3.0"
color_rgb = [160, 65, 60]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-ruby"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["superclass"]
hash_is_comment = true
is_executable = true
test_suffixes = ["_spec.rb", "_test.rb"]
test_prefixes = ["test_"]
test_dir_prefixes = ["spec/", "test/"]
test_dir_infixes = ["/spec/", "/test/"]
main_filenames = ["app.rb", "server.rb"]


[semantics.resolver]
alias_file = "*.gemspec"
alias_field = "spec.name"
source_root = "lib"
[semantics.project]
manifest_files = ["Gemfile"]
source_dirs = ["lib"]

[semantics.complexity]
branch_nodes = ["if", "unless", "elsif", "while", "until", "for", "case", "when", "rescue"]
logic_nodes = ["binary"]
logic_operators = ["&&", "||", "and", "or"]
nesting_nodes = ["if", "unless", "while", "until", "for", "case", "begin"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "elsif ", "unless ", "while ", "until ", "for ", "case ", "when ", "&&", "||"]
cog_branch = ["if ", "elsif ", "unless ", "while ", "until ", "for ", "case ", "when "]
"#,
r#"; Official tree-sitter-ruby tags.scm (v0.23.1)

; Method definitions
(
  (comment)* @doc
  .
  [
    (method
      name: (_) @name) @definition.method
    (singleton_method
      name: (_) @name) @definition.method
  ]
  (#strip! @doc "^#\\s*")
  (#select-adjacent! @doc @definition.method)
)

(alias
  name: (_) @name) @definition.method

(setter
  (identifier) @ignore)

; Class definitions
(
  (comment)* @doc
  .
  [
    (class
      name: [
        (constant) @name
        (scope_resolution
          name: (_) @name)
      ]) @definition.class
    (singleton_class
      value: [
        (constant) @name
        (scope_resolution
          name: (_) @name)
      ]) @definition.class
  ]
  (#strip! @doc "^#\\s*")
  (#select-adjacent! @doc @definition.class)
)

; Module definitions
(
  (module
    name: [
      (constant) @name
      (scope_resolution
        name: (_) @name)
    ]) @definition.module
)

; Calls
(call method: (identifier) @name) @reference.call

(
  [(identifier) (constant)] @name @reference.call
  (#is-not? local)
  (#not-match? @name "^(lambda|load|require|require_relative|__FILE__|__LINE__)$")
)

; ---- Import appendix (custom) ----

; require 'json' / require_relative './helper'
(call
  method: (identifier) @_method
  arguments: (argument_list
    (string) @import.module)
  (#match? @_method "^(require|require_relative)$")) @import
"#),
    ("rust",
r##"[plugin]
name = "rust"
display_name = "Rust"
version = "0.2.0"
extensions = ["rs"]
min_sentrux_version = "0.4.0"
color_rgb = [175, 135, 110]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-rust"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
package_index_files = ["mod.rs"]
is_executable = true
test_suffixes = ["_test.rs", "_tests.rs"]
test_dir_prefixes = ["tests/"]
test_dir_infixes = ["/tests/"]
test_module_kind = "mod_item"
test_attribute_kind = "attribute_item"
test_attribute_patterns = ["cfg", "test"]
entry_point_patterns = ["@main", "tokio::main", "actix_web::main", "#[main]"]
test_function_prefixes = ["test_", "bench_"]
implicit_entry_points = ["from", "into", "try_from", "try_into", "drop", "fmt", "clone", "eq", "hash", "cmp", "partial_cmp", "as_ref", "deref", "index"]
qualified_name_separator = "::"

[semantics.project]
manifest_files = ["Cargo.toml"]
ignored_dirs = ["target", ".cargo", ".rustup"]
source_dirs = ["src"]
mod_declaration_files = ["mod.rs", "lib.rs", "main.rs"]

[semantics.resolver]
alias_file = "Cargo.toml"
alias_field = "package.name"
alias_transform = "hyphen_to_underscore"
alias_entry_point = "src/lib.rs"
source_root = "src"
workspace_file = "Cargo.toml"
workspace_format = "toml"
workspace_members_field = "workspace.members"
workspace_package_name_field = "package.name"
workspace_entry_point = "src/lib.rs"

[semantics.import_ast]
strategy = "scoped_path"
path_separator = "::"
use_list_kind = "use_list"
scoped_path_kinds = ["scoped_identifier", "scoped_use_list"]
mod_declaration_kind = "mod_item"
leaf_identifier_kinds = ["identifier", "crate", "self"]
skip_type_imports_in_use_list = true

[semantics.complexity]
branch_nodes = ["if_expression", "else_clause", "for_expression", "while_expression", "loop_expression", "match_arm"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_expression", "for_expression", "while_expression", "loop_expression", "match_expression"]

[semantics.complexity_keywords]
cc = [" if ", "\tif ", "else if", "for ", "while ", "loop ", "&&", "||", "=> ", "=>{"]
cog_branch = ["if ", "else if", "for ", "while ", "loop ", "match "]
cog_nesting = ["if ", "for ", "while ", "loop ", "match "]
"##,
r#"; Official tree-sitter-rust tags.scm (v0.23.3)

; ADT definitions
(struct_item
    name: (type_identifier) @name) @definition.class

; Enums are algebraic data types (ADTs) — they provide polymorphic dispatch
; through pattern matching, equivalent to abstract classes/interfaces.
(enum_item
    name: (type_identifier) @name) @definition.adt

(union_item
    name: (type_identifier) @name) @definition.class

; type aliases
(type_item
    name: (type_identifier) @name) @definition.class

; method definitions
(declaration_list
    (function_item
        name: (identifier) @name) @definition.method)

; function definitions
(function_item
    name: (identifier) @name) @definition.function

; trait definitions
(trait_item
    name: (type_identifier) @name) @definition.interface

; module definitions
(mod_item
    name: (identifier) @name) @definition.module

; macro definitions
(macro_definition
    name: (identifier) @name) @definition.macro

; references
(call_expression
    function: (identifier) @name) @reference.call

(call_expression
    function: (field_expression
        field: (field_identifier) @name)) @reference.call

(macro_invocation
    macro: (identifier) @name) @reference.call

; implementations
(impl_item
    trait: (type_identifier) @name) @reference.implementation

(impl_item
    type: (type_identifier) @name
    !trait) @reference.implementation

; ---- Entry point: #[tokio::main] and similar attribute macros ----
(attribute_item) @entry

; ---- Import appendix (custom) ----

(use_declaration) @import

; mod declarations without body: `mod foo;` → import of sibling file
(mod_item
  !body) @import

; Scoped path calls: crate::module::func() or std::thread::spawn()
; The full scoped_identifier is captured as @call.scoped_path for implicit import extraction.
(call_expression
  function: (scoped_identifier
    name: (identifier) @call.name) @call.scoped_path) @call
"#),
    ("scala",
r#"[plugin]
name = "scala"
display_name = "Scala"
version = "0.1.0"
extensions = ["scala", "sc"]
min_sentrux_version = "0.3.0"
color_rgb = [155, 60, 75]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-scala"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
dot_is_module_separator = true
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["superclass", "super_interfaces", "type_list", "extends_type_clause", "class_type", "delegation_specifiers"]
is_executable = true
test_suffixes = ["Test.scala", "Spec.scala"]
test_dir_prefixes = ["test/", "tests/"]
test_dir_infixes = ["/test/", "/tests/"]


[semantics.resolver]
alias_file = "build.sbt"
alias_field = "name :="
source_root = "src/main/scala"
[semantics.project]
manifest_files = ["build.sbt"]
source_dirs = ["src"]

[semantics.import_ast]
strategy = "scoped_path"
path_separator = "."
scoped_path_kinds = ["scoped_identifier"]

[semantics.complexity]
branch_nodes = ["if_expression", "match_expression", "while_expression", "for_expression", "case_clause"]
logic_nodes = ["infix_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_expression", "match_expression", "while_expression", "for_expression", "try_expression"]
"#,
r#"; Scala structural queries

; Function definitions
(function_definition
  name: (identifier) @func.name) @func.def

; Class definitions
(class_definition
  name: (identifier) @class.name) @class.def

; Object definitions (singleton)
(object_definition
  name: (identifier) @class.name) @class.def

; Trait definitions
(trait_definition
  name: (identifier) @class.name) @class.def

; Imports
(import_declaration) @import

; Calls — direct
(call_expression
  function: (identifier) @call.name) @call

; Calls — field access  obj.method()
(call_expression
  function: (field_expression
    field: (identifier) @call.name)) @call
"#),
    ("scss",
r#"[plugin]
name = "scss"
display_name = "SCSS"
version = "0.1.0"
extensions = ["scss"]
min_sentrux_version = "0.3.0"
color_rgb = [155, 95, 125]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/serenadeai/tree-sitter-scss"
ref = "master"
abi_version = 14

[queries]
capabilities = ["imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = false

[semantics.project]
"#,
r#"; SCSS structural queries

; ---- Import appendix ----

; @import "file.scss"
(import_statement
  [(string_value) (call_expression)] @import.module) @import
"#),
    ("solidity",
r#"[plugin]
name = "solidity"
display_name = "Solidity"
version = "0.2.0"
extensions = ["sol"]
min_sentrux_version = "0.4.0"
color_rgb = [98, 126, 234]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/JoranHonig/tree-sitter-solidity"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = false
import_extractor = ""

[semantics.project]
manifest_files = ["hardhat.config.js", "hardhat.config.ts", "truffle-config.js", "foundry.toml"]
ignored_dirs = ["node_modules", "artifacts", "cache", "out"]
source_dirs = ["contracts", "src"]

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "while_statement"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "while_statement"]
"#,
r#"(function_definition) @definition.function
(contract_declaration) @definition.class
(interface_declaration) @definition.interface
(library_declaration) @definition.module
(struct_declaration) @definition.class
(event_definition) @definition.function
(import_directive) @import
"#),
    ("sql",
r#"[plugin]
name = "sql"
display_name = "SQL"
version = "0.2.0"
extensions = ["sql"]
min_sentrux_version = "0.4.0"
color_rgb = [220, 140, 60]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/DerekStride/tree-sitter-sql"
ref = "main"
abi_version = 14

[queries]
capabilities = []

[checksums]

[semantics]
is_executable = false
"#,
r#"; SQL — display-only language, no structural analysis
"#),
    ("svelte",
r#"[plugin]
name = "svelte"
display_name = "Svelte"
version = "0.2.0"
extensions = ["svelte"]
min_sentrux_version = "0.4.0"
color_rgb = [255, 62, 0]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-svelte"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
is_executable = false
import_extractor = ""

[semantics.project]
manifest_files = ["package.json"]
ignored_dirs = ["node_modules", ".svelte-kit", "build"]

[semantics.resolver]
alias_file = "package.json"
alias_field = "name"
source_root = "src"
"#,
r#"; Svelte tags.scm
; Svelte files contain embedded script elements with JS/TS

(script_element) @definition.module

(element
  (start_tag
    (tag_name) @name)) @reference.call

; ---- Import appendix ----
; Svelte imports live inside <script> blocks parsed as raw_text.

(script_element) @import
"#),
    ("swift",
r#"[plugin]
name = "swift"
display_name = "Swift"
version = "0.1.0"
extensions = ["swift"]
min_sentrux_version = "0.3.0"
color_rgb = [180, 80, 60]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/alex-pinkus/tree-sitter-swift"
ref = "main"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = true
test_suffixes = ["Tests.swift", "Test.swift"]
test_dir_prefixes = ["Tests/"]
test_dir_infixes = ["/Tests/"]
entry_point_patterns = ["@main", "@Main", "@UIApplicationMain"]


[semantics.resolver]
alias_file = "Package.swift"
alias_field = "name:"
source_root = "Sources"
[semantics.project]
manifest_files = ["Package.swift"]
source_dirs = ["Sources"]
implicit_module = true

[semantics.complexity]
branch_nodes = ["if_statement", "switch_statement", "for_statement", "while_statement", "catch_block"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "switch_statement", "for_statement", "while_statement"]
"#,
r#"; Based on official tree-sitter-swift tags.scm (v0.7.1)
; Modified: removed class_body method patterns (they assign class-level line numbers)
; Using standalone function/init patterns which give correct per-function line numbers.

(class_declaration
  name: (type_identifier) @name) @definition.class

(protocol_declaration
  name: (type_identifier) @name) @definition.interface

(function_declaration
    name: (simple_identifier) @name) @definition.function

; ---- Entry point: @main attribute on struct ----
(attribute (user_type (type_identifier) @_n)) @entry

; ---- Import appendix + calls (custom) ----

; Init declarations
(init_declaration) @func.def

; import Foundation / import UIKit.NSView
(import_declaration
  (identifier) @import.module) @import

; Calls — direct function call
(call_expression
  (simple_identifier) @call.name) @call

; Calls — navigation  object.method()
(call_expression
  (navigation_expression
    (navigation_suffix
      (simple_identifier) @call.name))) @call
"#),
    ("toml",
r#"[plugin]
name = "toml"
display_name = "TOML"
version = "0.1.0"
extensions = ["toml"]
color_rgb = [130, 75, 50]

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-toml"
ref = "master"
abi_version = 14

[queries]
capabilities = []

[checksums]

[semantics]
is_executable = false
"#,
r#"; TOML — display only, no structural analysis
"#),
    ("typescript",
r#"[plugin]
name = "typescript"
display_name = "TypeScript"
version = "0.1.0"
extensions = ["ts", "mts", "cts", "tsx"]
min_sentrux_version = "0.3.0"
color_rgb = [60, 110, 168]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter/tree-sitter-typescript"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
base_class_node_kinds = ["class_heritage", "extends_clause"]
package_index_files = ["index.ts", "index.tsx", "index.mts", "index.cts"]
is_executable = true
test_suffixes = [".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx"]
test_dir_prefixes = ["__tests__/", "test/", "tests/"]
test_dir_infixes = ["/__tests__/", "/test/", "/tests/"]
main_filenames = ["app.ts", "server.ts"]

[semantics.project]
manifest_files = ["package.json"]
ignored_dirs = ["node_modules", "dist", "build", ".next", "coverage"]
source_dirs = ["src", "lib", "packages"]

[semantics.resolver]
alias_file = "package.json"
alias_field = "name"
alias_entry_point = "src/index.ts"
path_alias_file = "tsconfig.json"
path_alias_field = "compilerOptions.paths"
path_alias_base_url = "compilerOptions.baseUrl"
resolve_extensions = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".mts", ".json"]
source_root = "src"
workspace_file = "package.json"
workspace_format = "json"
workspace_members_field = "workspaces"
workspace_package_name_field = "name"
workspace_entry_point = "src/index.ts"

[semantics.import_ast]
strategy = "field_read"
module_path_field = "source"
module_path_node_kinds = ["string"]
string_content_kind = "string_fragment"

[semantics.complexity]
branch_nodes = ["if_statement", "for_statement", "for_in_statement", "while_statement", "do_statement", "switch_statement", "catch_clause"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_statement", "for_statement", "for_in_statement", "while_statement", "do_statement", "switch_statement", "try_statement"]
"#,
r#"; Official tree-sitter-typescript tags.scm (v0.23.2) + inlined JS base patterns

; ---- TS-specific captures ----

(function_signature
  name: (identifier) @name) @definition.function

(method_signature
  name: (property_identifier) @name) @definition.method

(abstract_method_signature
  name: (property_identifier) @name) @definition.method

(abstract_class_declaration
  name: (type_identifier) @name) @definition.class

(module
  name: (identifier) @name) @definition.module

(interface_declaration
  name: (type_identifier) @name) @definition.interface

(new_expression
  constructor: (identifier) @name) @reference.class

; ---- JS base patterns (TS inherits JS grammar) ----

(
  (comment)* @doc
  .
  (method_definition
    name: (property_identifier) @name) @definition.method
  (#not-eq? @name "constructor")
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.method)
)

(
  (comment)* @doc
  .
  [
    (class
      name: (_) @name)
    (class_declaration
      name: (_) @name)
  ] @definition.class
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.class)
)

(
  (comment)* @doc
  .
  [
    (function_expression
      name: (identifier) @name)
    (function_declaration
      name: (identifier) @name)
    (generator_function
      name: (identifier) @name)
    (generator_function_declaration
      name: (identifier) @name)
  ] @definition.function
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.function)
)

(
  (comment)* @doc
  .
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name
      value: [(arrow_function) (function_expression)]) @definition.function)
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.function)
)

(
  (comment)* @doc
  .
  (variable_declaration
    (variable_declarator
      name: (identifier) @name
      value: [(arrow_function) (function_expression)]) @definition.function)
  (#strip! @doc "^[\\s\\*/]+|^[\\s\\*/]$")
  (#select-adjacent! @doc @definition.function)
)

(assignment_expression
  left: [
    (identifier) @name
    (member_expression
      property: (property_identifier) @name)
  ]
  right: [(arrow_function) (function_expression)]
) @definition.function

(pair
  key: (property_identifier) @name
  value: [(arrow_function) (function_expression)]) @definition.function

(
  (call_expression
    function: (identifier) @name) @reference.call
  (#not-match? @name "^(require)$")
)

(call_expression
  function: (member_expression
    property: (property_identifier) @name)
  arguments: (_) @reference.call)

; ---- Import appendix (custom) ----

(import_statement
  source: (string) @import.module) @import
"#),
    ("vlang",
r#"[plugin]
name = "vlang"
display_name = "V"
version = "0.2.0"
extensions = ["v"]
min_sentrux_version = "0.4.0"
color_rgb = [80, 130, 200]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[queries]
capabilities = ["functions", "classes", "imports"]

[checksums]

[semantics]
is_executable = true
import_extractor = ""

[semantics.project]
manifest_files = ["v.mod"]
ignored_dirs = [".vcache"]

[semantics.complexity]
branch_nodes = ["if_expression", "for_statement", "match_expression", "else_branch"]
logic_nodes = ["binary_expression"]
logic_operators = ["&&", "||"]
nesting_nodes = ["if_expression", "for_statement", "match_expression"]
"#,
r#"; V language tags.scm

(function_declaration
  name: (identifier) @name) @definition.function

(struct_declaration
  name: (identifier) @name) @definition.class

(enum_declaration
  name: (identifier) @name) @definition.class

(interface_declaration
  name: (identifier) @name) @definition.interface

; ---- Import appendix ----

(import_declaration
  (import_path) @import.module) @import
"#),
    ("vue",
r#"[plugin]
name = "vue"
display_name = "Vue"
version = "0.2.0"
extensions = ["vue"]
min_sentrux_version = "0.4.0"
color_rgb = [65, 184, 131]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-vue"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions", "imports"]

[checksums]

[semantics]
is_executable = false
import_extractor = ""

[semantics.project]
manifest_files = ["package.json"]
ignored_dirs = ["node_modules", "dist"]

[semantics.resolver]
alias_file = "package.json"
alias_field = "name"
source_root = "src"
"#,
r#"; Vue tags.scm
; Vue SFC files contain template, script, and style sections

(script_element) @definition.module

(element
  (start_tag
    (tag_name) @name)) @reference.call

; ---- Import appendix ----
; Vue imports live inside <script> blocks parsed as raw_text.

(script_element) @import
"#),
    ("yaml",
r#"[plugin]
name = "yaml"
display_name = "YAML"
version = "0.2.0"
extensions = ["yaml", "yml"]
min_sentrux_version = "0.4.0"
color_rgb = [155, 50, 55]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-yaml"
ref = "master"
abi_version = 14

[queries]
capabilities = []

[checksums]

[semantics]
is_executable = false
"#,
r#"; YAML — display-only language, no structural analysis
"#),
    ("zig",
r#"[plugin]
name = "zig"
display_name = "Zig"
version = "0.1.0"
extensions = ["zig"]
min_sentrux_version = "0.3.0"
color_rgb = [180, 135, 60]

[plugin.metadata]
author = "sentrux"
license = "MIT"

[grammar]
source = "https://github.com/tree-sitter-grammars/tree-sitter-zig"
ref = "master"
abi_version = 14

[queries]
capabilities = ["functions"]

[checksums]

[semantics]
import_extractor = ""
base_class_extractor = "generic"
is_executable = true

[semantics.project]

[semantics.complexity]
branch_nodes = ["if_expression", "switch_expression", "while_expression", "for_expression"]
logic_nodes = []
logic_operators = []
nesting_nodes = ["if_expression", "switch_expression", "while_expression", "for_expression"]
"#,
r#"; Zig structural queries (hand-written, no official tags.scm)

; Function declarations
(function_declaration
  name: (identifier) @func.name) @func.def

; Test declarations
(test_declaration
  (identifier) @func.name) @func.def

(test_declaration
  (string) @func.name) @func.def
"#),
];