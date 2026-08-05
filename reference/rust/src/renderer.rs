//! Mock renderer for the reference implementation.
//!
//! This is intentionally minimal — a renderer is just a function:
//! `fn render(blob: &Blob) -> String`. In a real browser engine (like
//! Robinson or Servo), this would be HTML parsing → CSS layout → paint.
//! Here it's a string template. The point is that a renderer is
//! **stateless** — it transforms a blob into output, with no side effects.

use crate::state_blob::Blob;

/// A minimal HTML renderer.
///
/// Renders a blob's `payload` as an HTML string. This is the simplest
/// possible renderer — it proves the concept without any complexity.
pub struct HtmlRenderer;

impl HtmlRenderer {
    /// Render a blob to an HTML string.
    ///
    /// Looks for `title`, `body`, and `items` in the payload.
    pub fn render(blob: &Blob) -> String {
        let title = blob
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let body = blob
            .payload
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let items = blob.payload.get("items").and_then(|v| v.as_array());

        let mut html = String::with_capacity(256);
        html.push_str("<!DOCTYPE html><html><head><title>");
        html.push_str(title);
        html.push_str("</title></head><body>");
        html.push_str("<h1>");
        html.push_str(title);
        html.push_str("</h1>");
        html.push_str("<p>");
        html.push_str(body);
        html.push_str("</p>");

        if let Some(items) = items {
            html.push_str("<ul>");
            for item in items {
                html.push_str("<li>");
                if let Some(s) = item.as_str() {
                    html.push_str(s);
                } else {
                    html.push_str(&item.to_string());
                }
                html.push_str("</li>");
            }
            html.push_str("</ul>");
        }

        html.push_str("</body></html>");
        html
    }
}

/// Render a blob as plain text (for non-HTML contexts).
pub struct TextRenderer;

impl TextRenderer {
    pub fn render(blob: &Blob) -> String {
        let title = blob
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let body = blob
            .payload
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        format!("{}\n\n{}", title, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_renderer_basic() {
        let blob = Blob::new(
            "test://page",
            serde_json::json!({ "title": "My Page", "body": "Hello World" }),
        );
        let html = HtmlRenderer::render(&blob);
        assert!(html.contains("<title>My Page</title>"));
        assert!(html.contains("<h1>My Page</h1>"));
        assert!(html.contains("<p>Hello World</p>"));
    }

    #[test]
    fn html_renderer_with_items() {
        let blob = Blob::new(
            "test://page",
            serde_json::json!({
                "title": "List",
                "body": "My items:",
                "items": ["one", "two", "three"]
            }),
        );
        let html = HtmlRenderer::render(&blob);
        assert!(html.contains("<li>one</li>"));
        assert!(html.contains("<li>two</li>"));
        assert!(html.contains("<li>three</li>"));
    }

    #[test]
    fn html_renderer_defaults() {
        let blob = Blob::new("test://page", serde_json::json!({}));
        let html = HtmlRenderer::render(&blob);
        assert!(html.contains("<title>Untitled</title>"));
    }

    #[test]
    fn text_renderer_basic() {
        let blob = Blob::new(
            "test://page",
            serde_json::json!({ "title": "My Page", "body": "Hello World" }),
        );
        let text = TextRenderer::render(&blob);
        assert!(text.contains("My Page"));
        assert!(text.contains("Hello World"));
    }
}
