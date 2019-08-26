use syntect::dumps::from_binary;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{
    append_highlighted_html_for_styled_line, start_highlighted_html_snippet, IncludeBackground,
};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

lazy_static! {
    pub static ref SYNTAX_SET: SyntaxSet = {
        let syntax_set: SyntaxSet = from_binary(include_bytes!("../syntect/syntaxes.packdump"));
        syntax_set
    };
    pub static ref THEME_SET: ThemeSet = {
        let theme_set: ThemeSet = from_binary(include_bytes!("../syntect/themes.themedump"));
        theme_set
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

// let mut html_generator = syntect::html::ClassedHTMLGenerator::new(syntax, &SYNTAX_SET);
//     for line in LinesWithEndings::from(&string) {
//         html_generator.parse_html_for_line(&line)
//     }
//     html_generator.finalize()

pub fn highlight(token: &str, string: &str) -> String {
    let syntax = get_syntax(token, string);
    let theme = &THEME_SET.themes["lychee"];
    let mut highlighter = HighlightLines::new(syntax, theme);
    let (_, bg) = start_highlighted_html_snippet(theme);
    let mut output = String::from("<ol class=\"blob-content__lines\">");

    for line in LinesWithEndings::from(string) {
        let regions = highlighter.highlight(line, &SYNTAX_SET);
        output += "<li class=\"blob-content__line\">";
        append_highlighted_html_for_styled_line(
            &regions[..],
            IncludeBackground::IfDifferent(bg),
            &mut output,
        );
    }

    output += "</ol>";
    output
}
