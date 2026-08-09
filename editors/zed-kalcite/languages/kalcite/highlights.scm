(comment) @comment
(string) @string
(number) @number
(identifier) @variable
(class_declaration name: (identifier) @type)
(struct_declaration name: (identifier) @type)
(function_declaration name: (identifier) @function)
["class" "struct" "fn" "var" "const" "return" "extends"] @keyword
"@" @attribute
