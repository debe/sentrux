; SCSS structural queries

; Mixins  @mixin name { ... }
(mixin_statement
  name: (identifier) @func.name) @func.def

; Functions  @function name() { ... }
(function_statement
  name: (identifier) @func.name) @func.def

; Includes  @include name;
(include_statement
  (identifier) @call.name) @call

; ---- Import appendix ----

; @import "file.scss"
(import_statement
  [(string_value) (call_expression)] @import.module) @import
