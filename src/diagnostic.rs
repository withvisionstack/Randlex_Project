#[derive(Debug)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub length: usize,
    pub helps: Vec<String>,
}

impl Diagnostic {
    pub fn new(code: &'static str, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self { code, message: message.into(), line, column, length: 1, helps: Vec::new() }
    }

    pub fn length(mut self, length: usize) -> Self {
        self.length = length;
        self
    }

    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.helps.push(text.into());
        self
    }

    pub fn render(&self, file: &str, source: &str, development: bool) -> String {
        let line_text = source.lines().nth(self.line - 1).unwrap_or("");
        let number = self.line.to_string();
        let pad = " ".repeat(number.len());

        let mut out = String::new();
        out += &format!("error[{}]: {}\n", self.code, self.message);
        out += &format!("{pad}--> {}:{}:{}\n", file, self.line, self.column);
        out += &format!("{pad} |\n");
        out += &format!("{number} | {line_text}\n");
        out += &format!(
            "{pad} | {}{}\n",
            " ".repeat(self.column.saturating_sub(1)),
            "^".repeat(self.length.max(1))
        );
        for help in &self.helps {
            out += &format!("{pad} = help: {help}\n");
        }

        match (development, explain(self.code)) {
            (true, Some(text)) => out += &format!("\n{text}\n"),
            (false, Some(_)) => out += &format!(
                "\nFor more information about this error, run with --development or: rendlex --explain {}\n",
                self.code
            ),
            _ => {}
        }
        out
    }
}

// Catálogo de erros: a explicação longa de cada código
pub fn explain(code: &str) -> Option<&'static str> {
    match code {
        "RX001" => Some(
            "RX001: unexpected character

The lexer found a character that is not part of Rendlex syntax.
A common cause is a stray '\\' outside of a string.

Wrong:   Show \\\"Hello\\\";
Correct: Show \"Hello\";"
        ),

        "RX002" => Some(
            "RX002: unterminated string

A string was opened with '\"' but never closed.

Wrong:   Show \"Hello;
Correct: Show \"Hello\";"
        ),

        "RX003" => Some(
            "RX003: invalid escape sequence

Inside strings only \\n, \\t, \\\" and \\\\ are accepted."
        ),

        "RX004" => Some(
            "RX004: expected a variable name after '$'

Variables look like $name. The name must start with a letter or '_'."
        ),

        "RX100" => Some(
            "RX100: syntax error

The parser found a token that does not fit here.
Check for a missing ';', ')' or '}'."
        ),

        "RX101" => Some(
            "RX101: constant declared after code

All constants must be at the top of the file,
before any statement."
        ),

        "RX102" => Some(
            "RX102: constant name must be UPPERCASE

Constants must use only uppercase letters.

Wrong:   const max_size: i64 = 10;
Correct: const MAX_SIZE: i64 = 10;"
        ),

        "RX103" => Some(
            "RX103: 'break' or 'continue' outside a loop

They only make sense inside while/for.
To end a whole task, use 'stop'."
        ),

        "RX300" => Some(
            "RX300: unknown type

Valid types:
int8 int16 int32 int64
uint8 uint16 uint32 uint64
float32 float64
str bool"
        ),

        "RX301" => Some(
            "RX301: variable already declared

The type is fixed on first use.

Use '$x = value;' to change only the value."
        ),

        "RX302" => Some(
            "RX302: mismatched types

A variable keeps its type forever.

Wrong:   $x: int8 = 6;
         $x = \"abc\";

Correct: $x: int8 = 6;
         $x = 7;"
        ),

        "RX303" => Some(
            "RX303: literal out of range

The number does not fit in the chosen type.

Example:
int8 accepts values from -128 to 127."
        ),

        "RX304" => Some(
            "RX304: undefined name

Use a variable only after assigning it."
        ),

        "RX305" => Some(
            "RX305: invalid operator for this type

Example:
you cannot subtract strings
or negate a bool."
        ),

        "RX306" => Some(
            "RX306: variable used before being declared

A variable must be declared with an explicit type before it is assigned.

Wrong:   $total = 200;
Correct: $total: int64 = 200;
         $total = 300;   // later changes are fine"),

        _ => None,
    }
}