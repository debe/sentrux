; HTML structural queries

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
