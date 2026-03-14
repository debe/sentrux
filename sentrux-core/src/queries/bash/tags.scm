; Bash structural queries

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
