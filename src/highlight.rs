use syntect::dumps::from_binary;
use syntect::html::{tokens_to_classed_spans, ClassStyle};
use syntect::parsing::{ParseState, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

lazy_static! {
    pub static ref SYNTAX_SET: SyntaxSet = {
        let syntax_set: SyntaxSet = from_binary(include_bytes!("../syntaxes.packdump"));
        syntax_set
    };
}

pub fn get_syntax<'string>(token: &str, string: &'string str) -> &'string SyntaxReference {
    SYNTAX_SET.find_syntax_by_token(token).unwrap_or_else(|| {
        if token.len() > 1 {
            match SYNTAX_SET.find_syntax_by_token(&token[1..]) {
                Some(syntax) => return syntax,
                _ => {}
            }
        }
        let mut lines = string.lines();
        let first_line = lines.next();
        SYNTAX_SET
            .find_syntax_by_first_line(first_line.unwrap_or(""))
            .unwrap_or(SYNTAX_SET.find_syntax_plain_text())
    })
}

pub fn highlight(token: &str, string: &str) -> String {
    let syntax = get_syntax(token, string);
    let mut parse_state = ParseState::new(syntax);
    let mut result = String::new();
    for line in LinesWithEndings::from(string) {
        let ops = parse_state.parse_line(line, &SYNTAX_SET);
        let (formatted_line, _) = tokens_to_classed_spans(line, ops.as_slice(), ClassStyle::Spaced);
        result += &formatted_line;
    }
    return result;
}
