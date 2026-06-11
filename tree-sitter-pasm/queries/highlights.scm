(keyword) @keyword
(operator) @function.builtin
(typed_word) @function.builtin

(type_literal) @type
(variadic_type) @type
(boolean) @boolean
(number) @number
(string) @string
(char) @character
(comment) @comment
(identifier) @variable
(element_call) @function.call

[
  (arrow)
  (pipe)
  (range_operator)
  (fallback)
] @operator

(function_literal
  (signature) @function)

(guard
  (keyword) @keyword.conditional)
