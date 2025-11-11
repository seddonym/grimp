use std::{fs, io::Read, path::Path};

use encoding_rs::Encoding;

use crate::errors::{GrimpError, GrimpResult};

/// Read a Python source file with proper encoding detection.
///
/// Python PEP 263 specifies that encoding can be declared in the first or second line
/// in the format: `# coding: <encoding>` or `# -*- coding: <encoding> -*-`
///
/// This function:
/// 1. Reads the file as bytes
/// 2. Checks the first two lines for an encoding declaration
/// 3. Decodes the file using the detected encoding (or UTF-8 as default)
pub fn read_python_file(path: &Path) -> GrimpResult<String> {
    // Read file as bytes
    let mut file = fs::File::open(path).map_err(|e| GrimpError::FileReadError {
        path: path.display().to_string(),
        error: e.to_string(),
    })?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| GrimpError::FileReadError {
            path: path.display().to_string(),
            error: e.to_string(),
        })?;

    // Detect encoding from first two lines
    let encoding = detect_python_encoding(&bytes);

    // Decode using detected encoding
    let (decoded, _encoding_used, had_errors) = encoding.decode(&bytes);

    if had_errors {
        return Err(GrimpError::FileReadError {
            path: path.display().to_string(),
            error: format!("Failed to decode file with encoding {}", encoding.name()),
        });
    }

    Ok(decoded.into_owned())
}

/// Detect Python source file encoding from the first two lines.
///
/// Looks for patterns like:
/// - `# coding: <encoding>`
/// - `# -*- coding: <encoding> -*-`
/// - `# coding=<encoding>`
fn detect_python_encoding(bytes: &[u8]) -> &'static Encoding {
    // Read first two lines as ASCII (encoding declarations must be ASCII-compatible)
    let mut line_count = 0;
    let mut line_start = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if byte == b'\n' {
            line_count += 1;
            if line_count <= 2 {
                // Check this line for encoding declaration
                let line = &bytes[line_start..i];
                if let Some(encoding) = extract_encoding_from_line(line) {
                    return encoding;
                }
                line_start = i + 1;
            } else {
                break;
            }
        }
    }

    // Default to UTF-8
    encoding_rs::UTF_8
}

/// Extract encoding from a single line if it contains an encoding declaration.
fn extract_encoding_from_line(line: &[u8]) -> Option<&'static Encoding> {
    // Convert line to string (should be ASCII for encoding declarations)
    let line_str = std::str::from_utf8(line).ok()?;

    // Look for "coding:" or "coding="
    if let Some(pos) = line_str
        .find("coding:")
        .or_else(|| line_str.find("coding="))
    {
        let after_coding = &line_str[pos + 7..]; // Skip "coding:" or "coding="

        // Extract encoding name (alphanumeric, dash, underscore until whitespace or special char)
        let encoding_name: String = after_coding
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();

        if !encoding_name.is_empty() {
            // Try to get the encoding
            return Encoding::for_label(encoding_name.as_bytes());
        }
    }

    None
}
