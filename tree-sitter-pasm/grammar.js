module.exports = grammar({
  name: 'pasm',

  extras: $ => [
    /\s/,
    $.comment,
  ],

  word: $ => $.identifier,

  conflicts: $ => [
    [$._form, $._pattern],
    [$._form, $._pattern, $.range_pattern],
    [$.list, $.list_pattern],
    [$.list, $.destructure_pattern],
  ],

  rules: {
    source_file: $ => repeat($._form),

    _form: $ => choice(
      $.function_literal,
      $.list,
      $.block,
      $.typed_word,
      $.keyword,
      $.operator,
      $.type_literal,
      $.boolean,
      $.string,
      $.char,
      $.number,
      $.identifier,
      $.element_call,
      $.fallback,
      $.range_operator,
      $.pipe,
      $.arrow,
    ),

    comment: _ => token(seq(';;', /[^\n]*/)),

    function_literal: $ => seq(
      $.signature,
      optional($.guard),
      $.block,
    ),

    signature: $ => seq(
      field('inputs', $.pattern_list),
      $.arrow,
      field('outputs', $.pattern_list),
    ),

    pattern_list: $ => seq(
      '(',
      repeat($._pattern),
      ')',
    ),

    guard: $ => seq(
      field('keyword', alias('when', $.keyword)),
      $.block,
    ),

    block: $ => seq(
      '{',
      repeat($._form),
      '}',
    ),

    list: $ => seq(
      '[',
      repeat(choice($._form, $._pattern)),
      ']',
    ),

    _pattern: $ => choice(
      $.type_literal,
      $.variadic_type,
      $.fallback,
      $.range_pattern,
      $.destructure_pattern,
      $.list_pattern,
      $.string,
      $.char,
      $.number,
      $.boolean,
      $.identifier,
    ),

    list_pattern: $ => seq(
      '[',
      repeat($._pattern),
      ']',
    ),

    destructure_pattern: $ => seq(
      '[',
      $._pattern,
      $.pipe,
      $._pattern,
      ']',
    ),

    range_pattern: $ => seq(
      $.number,
      $.range_operator,
      $.number,
    ),

    variadic_type: _ => token(seq('..@', /[A-Za-z_][A-Za-z0-9_-]*/)),
    type_literal: _ => token(seq('@', /[A-Za-z_][A-Za-z0-9_-]*/)),

    keyword: _ => token(choice(
      'push',
      'drop',
      'clear',
      'into',
      'take',
      'delete',
      'call',
      'eval',
      'ret',
      'if',
      'else',
      'when',
      'include',
      'match',
      'quit',
    )),

    typed_word: _ => token(choice(
      'int?',
      'string?',
      'bool?',
      'char?',
      'type?',
    )),

    operator: _ => token(choice(
      'add',
      'sub',
      'mul',
      'div',
      'neg',
      'dup',
      'swap',
      'rot',
      'over',
      'roll',
      'pick',
      'eq',
      'lt',
      'gt',
      'and',
      'or',
      'not',
      'len',
      'stack-len',
      'concat',
      'cons',
      'uncon',
      'at',
      'explode',
      'pack',
      'first',
      'last',
      'find?',
      'substr?',
      'sys-open',
      'sys-close',
      'sys-read',
      'sys-write',
    )),

    boolean: _ => token(choice('true', 'false')),
    number: _ => token(prec(2, seq(optional('-'), /\d+/))),
    string: _ => token(seq('"', repeat(choice(/[^"\\]/, /\\./)), '"')),
    char: _ => token(seq("'", choice(/[^'\\]/, /\\./), "'")),
    element_call: _ => token(seq('#', /[A-Za-z0-9_][A-Za-z0-9_?!-]*/)),
    identifier: _ => token(prec(1, /[A-Za-z0-9_][A-Za-z0-9_?!-]*/)),
    fallback: _ => token('..'),
    range_operator: _ => token('..<'),
    pipe: _ => token('|'),
    arrow: _ => token('->'),
  },
});
