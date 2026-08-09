module.exports = grammar({
  name: 'kalcite',
  extras: $ => [/\s/, $.comment],
  word: $ => $.identifier,
  rules: {
    source_file: $ => repeat($._declaration),
    comment: _ => token(choice(seq('//', /.*/), seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'))),
    _declaration: $ => choice($.use_declaration, $.class_declaration, $.struct_declaration, $.function_declaration, $.field_declaration, $.signal_declaration),
    use_declaration: $ => seq('use', $.identifier, repeat(seq('.', $.identifier)), ';'),
    attributes: $ => repeat1(seq('@', $.identifier, optional(seq('(', optional(commaSep($._expression)), ')')))),
    class_declaration: $ => seq(optional($.attributes), 'class', field('name', $.identifier), optional(seq('extends', $.identifier)), $.block),
    struct_declaration: $ => seq(optional($.attributes), 'struct', field('name', $.identifier), $.block),
    function_declaration: $ => seq(optional($.attributes), 'fn', field('name', $.identifier), '(', optional(commaSep($.parameter)), ')', optional(seq('->', $._type)), $.block),
    parameter: $ => seq($.identifier, ':', $._type),
    field_declaration: $ => seq(optional($.attributes), choice('var', 'const'), $.identifier, ':', $._type, optional(seq('=', $._expression)), ';'),
    signal_declaration: $ => seq(optional($.attributes), 'signal', field('name', $.identifier), '(', optional(commaSep($.parameter)), ')', ';'),
    block: $ => seq('{', repeat(choice($._declaration, $.statement)), '}'),
    statement: $ => choice($.native_statement, $.local_declaration, seq($._expression, ';'), $.block, seq('return', optional($._expression), ';')),
    native_statement: $ => seq('unsafe', choice('rust', 'asm'), optional(seq('[', $.identifier, ']')), '{', optional($.native_body), '}'),
    native_body: _ => token(/[^}]+/),
    local_declaration: $ => choice(
      seq('var', $.identifier, optional(seq(':', $._type)), optional(seq('=', $._expression)), ';'),
      seq('const', choice(seq($._type, $.identifier), seq($.identifier, optional(seq(':', $._type)))), optional(seq('=', $._expression)), ';'),
      seq($._type, $.identifier, optional(seq('=', $._expression)), ';')
    ),
    _expression: $ => choice($.identifier, $.number, $.string, $.array, $.call),
    call: $ => prec(1, seq($.identifier, '(', optional(commaSep($._expression)), ')')),
    array: $ => seq('[', optional(commaSep($._expression)), ']'),
    _type: $ => choice(
      $.identifier,
      seq('[', $._type, ';', $.number, ']'),
      seq($.identifier, '[', $._type, optional(seq(';', $.number)), ']')
    ),
    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,
    number: _ => /0x[0-9A-Fa-f_]+|0b[01_]+|[0-9][0-9_]*/,
    string: _ => /"([^"\\]|\\.)*"/,
  }
});
function commaSep(rule) { return seq(rule, repeat(seq(',', rule)), optional(',')); }
