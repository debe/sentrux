; CSS structural queries
; CSS has no functions/classes in the traditional sense

; ---- Import appendix ----

; @import "file.css" or @import url("file.css")
; Capture the string value as import.module
(import_statement
  [(string_value) (call_expression)] @import.module) @import
