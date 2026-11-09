//! Turn article HTML into plain text for the content panel.
//!
//! Full articles (RSS `<content:encoded>`, Atom `<content>`) are long HTML
//! documents, so this does a little more than stripping tags: block elements
//! become line breaks, `<pre>` keeps its whitespace, `<script>`/`<style>`
//! bodies are dropped and entities are decoded after tags are removed, so
//! escaped markup such as `&lt;Vec&gt;` stays visible.

use std::borrow::Cow;

/// Convert an article body (HTML or plain text) into readable plain text.
///
/// Paragraphs are separated by a blank line, list items start with `• `,
/// and runs of blank lines are collapsed. Input without any tags is treated
/// as plain text and keeps its own line breaks.
pub fn html_to_text(html: &str) -> String {
    if !looks_like_html(html) {
        let mut text = TextBuilder::default();
        text.push_raw(&decode(html));
        return text.finish();
    }

    let mut text = TextBuilder::default();
    let mut rest = html;

    while let Some(start) = rest.find('<') {
        text.push_text(&rest[..start]);
        rest = &rest[start..];

        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
            continue;
        }

        let Some(end) = rest.find('>').filter(|_| starts_tag(rest)) else {
            // A bare `<` that does not open a tag is just text.
            text.push_text("<");
            rest = &rest[1..];
            continue;
        };

        let tag = Tag::parse(&rest[1..end]);
        rest = &rest[end + 1..];

        if !tag.closing && !tag.self_closing && is_raw_text_element(&tag.name) {
            rest = skip_past_closing_tag(rest, &tag.name);
            continue;
        }

        text.apply(&tag);
    }

    text.push_text(rest);
    text.finish()
}

/// Whether `s` contains something that looks like an HTML tag or comment.
fn looks_like_html(s: &str) -> bool {
    s.match_indices('<').any(|(i, _)| starts_tag(&s[i..]))
}

/// Whether `s` (starting at a `<`) opens a tag, closing tag or comment.
fn starts_tag(s: &str) -> bool {
    s[1..]
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!')
}

/// Elements whose contents are not article text.
fn is_raw_text_element(name: &str) -> bool {
    matches!(
        name,
        "script" | "style" | "head" | "template" | "noscript" | "svg"
    )
}

/// Skip to just after `</name ...>`, or to the end if it never closes.
fn skip_past_closing_tag<'a>(s: &'a str, name: &str) -> &'a str {
    for (i, _) in s.match_indices("</") {
        let after = &s[i + 2..];
        let matches_name = after
            .get(..name.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name));
        if matches_name {
            return after.find('>').map_or("", |end| &after[end + 1..]);
        }
    }
    ""
}

fn decode(s: &str) -> Cow<'_, str> {
    html_escape::decode_html_entities(s)
}

/// The parts of a tag the converter cares about.
struct Tag {
    /// Lowercase element name, empty for `<!DOCTYPE>` and the like.
    name: String,
    closing: bool,
    self_closing: bool,
}

impl Tag {
    /// Parse the text between `<` and `>`.
    fn parse(inner: &str) -> Self {
        let (closing, body) = inner
            .strip_prefix('/')
            .map_or((false, inner), |body| (true, body));
        let name = body
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase();
        Self {
            name,
            closing,
            self_closing: inner.trim_end().ends_with('/'),
        }
    }
}

/// Accumulates text with HTML-like whitespace handling.
#[derive(Default)]
struct TextBuilder {
    out: String,
    /// Whitespace seen since the last word, emitted lazily as one space.
    pending_space: bool,
    /// Depth of open `<pre>` elements.
    pre_depth: usize,
    /// A list bullet was written and no text has followed it yet, so block
    /// tags inside the `<li>` (like `<p>`) must not break the line.
    bullet_open: bool,
}

impl TextBuilder {
    /// Append text from between tags.
    fn push_text(&mut self, raw: &str) {
        if raw.is_empty() {
            return;
        }
        let decoded = decode(raw);
        if self.pre_depth > 0 {
            self.push_raw(&decoded);
            return;
        }
        for c in decoded.chars() {
            if c.is_ascii_whitespace() {
                self.pending_space = true;
            } else {
                if self.pending_space && !self.at_line_start() && !self.bullet_open {
                    self.out.push(' ');
                }
                self.pending_space = false;
                self.bullet_open = false;
                // A non-breaking space would stop the terminal from wrapping.
                self.out.push(if c == '\u{a0}' { ' ' } else { c });
            }
        }
    }

    /// Append text as is, keeping its line breaks and indentation.
    fn push_raw(&mut self, text: &str) {
        self.pending_space = false;
        self.bullet_open &= text.trim().is_empty();
        self.out
            .push_str(&text.replace("\r\n", "\n").replace('\u{a0}', " "));
    }

    fn at_line_start(&self) -> bool {
        self.out.is_empty() || self.out.ends_with('\n')
    }

    /// Start a new line unless already at the start of one.
    fn line_break(&mut self) {
        self.pending_space = false;
        if !self.at_line_start() && !self.bullet_open {
            self.trim_trailing_spaces();
            self.out.push('\n');
        }
    }

    /// Always end the current line, so `<br><br>` leaves a blank line.
    fn forced_line_break(&mut self) {
        self.pending_space = false;
        self.bullet_open = false;
        self.trim_trailing_spaces();
        self.out.push('\n');
    }

    /// Separate blocks with one blank line.
    fn paragraph_break(&mut self) {
        if self.bullet_open {
            return;
        }
        self.line_break();
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
    }

    fn trim_trailing_spaces(&mut self) {
        let trimmed = self.out.trim_end_matches(' ').len();
        self.out.truncate(trimmed);
    }

    /// Apply the layout effect of a tag.
    fn apply(&mut self, tag: &Tag) {
        match tag.name.as_str() {
            "br" => self.forced_line_break(),
            "pre" => {
                self.paragraph_break();
                if tag.closing {
                    self.pre_depth = self.pre_depth.saturating_sub(1);
                } else if !tag.self_closing {
                    self.pre_depth += 1;
                }
            }
            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "ul" | "ol"
            | "dl" | "table" | "figure" | "figcaption" | "section" | "article" | "header"
            | "footer" | "aside" | "nav" | "main" | "hr" | "details" => {
                self.paragraph_break();
            }
            "li" => {
                self.bullet_open = false;
                self.line_break();
                if !tag.closing {
                    self.out.push_str("• ");
                    self.bullet_open = true;
                }
            }
            "tr" | "dt" | "dd" | "summary" => self.line_break(),
            "td" | "th" => self.pending_space = true,
            _ => {}
        }
    }

    /// Trim line ends, collapse runs of blank lines and trim the ends.
    fn finish(self) -> String {
        let mut result = String::with_capacity(self.out.len());
        let mut blank_run = 0;
        for line in self.out.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                blank_run += 1;
                if blank_run > 1 {
                    continue;
                }
            } else {
                blank_run = 0;
            }
            result.push_str(line);
            result.push('\n');
        }
        result.trim_matches('\n').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_with_attributes_should_become_separate_paragraphs() {
        let text = html_to_text(r#"<p class="lead">One</p><p style="x">Two</p>"#);

        assert_eq!(text, "One\n\nTwo");
    }

    #[test]
    fn headings_and_list_items_should_get_their_own_lines() {
        let text = html_to_text("<h2>Title</h2><ul><li>first</li><li>second</li></ul><p>After</p>");

        assert_eq!(text, "Title\n\n• first\n• second\n\nAfter");
    }

    #[test]
    fn paragraphs_inside_list_items_should_stay_on_the_bullet_line() {
        let text = html_to_text("<ul><li><p>Loose item</p></li><li><p>Second</p></li></ul>");

        assert_eq!(text, "• Loose item\n\n• Second");
    }

    #[test]
    fn indented_list_markup_should_not_add_space_after_the_bullet() {
        let text = html_to_text("<ul>\n  <li>\n    <p>Spaced</p>\n  </li>\n</ul>");

        assert_eq!(text, "• Spaced");
    }

    #[test]
    fn script_and_style_contents_should_be_dropped() {
        let text = html_to_text(
            "<p>Hello</p><script>alert('x < y')</script><STYLE>p { color: red }</STYLE><p>World</p>",
        );

        assert_eq!(text, "Hello\n\nWorld");
    }

    #[test]
    fn named_and_numeric_entities_should_be_decoded() {
        let text = html_to_text("<p>It&#8217;s &mdash; fine&hellip; &amp; &#x2014; done</p>");

        assert_eq!(text, "It\u{2019}s \u{2014} fine\u{2026} & \u{2014} done");
    }

    #[test]
    fn escaped_markup_should_stay_visible_as_text() {
        let text = html_to_text("<p>Use &lt;Vec&lt;T&gt;&gt; here</p>");

        assert_eq!(text, "Use <Vec<T>> here");
    }

    #[test]
    fn source_whitespace_should_collapse_like_a_browser() {
        let text = html_to_text("<p>one\n    two</p>\n\n\n   <p>  three </p>");

        assert_eq!(text, "one two\n\nthree");
    }

    #[test]
    fn preformatted_text_should_keep_its_whitespace() {
        let text = html_to_text("<p>Code:</p><pre><code>fn main() {\n    run();\n}</code></pre>");

        assert_eq!(text, "Code:\n\nfn main() {\n    run();\n}");
    }

    #[test]
    fn line_breaks_should_start_new_lines() {
        let text = html_to_text("a<br>b<br/>c<BR />d");

        assert_eq!(text, "a\nb\nc\nd");
    }

    #[test]
    fn comments_should_be_removed() {
        let text = html_to_text("<p>Before<!-- <p>hidden</p> -->After</p>");

        assert_eq!(text, "BeforeAfter");
    }

    #[test]
    fn plain_text_should_keep_its_line_breaks() {
        let text = html_to_text("Line one\nLine two\n\n\n\nLine three &amp; more");

        assert_eq!(text, "Line one\nLine two\n\nLine three & more");
    }

    #[test]
    fn a_lone_less_than_sign_should_stay_in_the_text() {
        let text = html_to_text("<p>1 < 2 and 3 > 2</p>");

        assert_eq!(text, "1 < 2 and 3 > 2");
    }
}
