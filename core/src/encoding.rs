/// Encoding and newline preservation
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Latin1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Newline {
    Lf,
    Crlf,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub encoding: Encoding,
    pub newline: Newline,
    pub has_bom: bool,
}

impl FileMetadata {
    /// Detect encoding and newline from file content
    pub fn detect(content: &[u8]) -> Self {
        let (encoding, has_bom) = Self::detect_encoding(content);
        let newline = Self::detect_newline(content);
        
        Self {
            encoding,
            newline,
            has_bom,
        }
    }
    
    fn detect_encoding(content: &[u8]) -> (Encoding, bool) {
        // Check for BOM
        if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return (Encoding::Utf8Bom, true);
        }
        
        if content.starts_with(&[0xFF, 0xFE]) {
            return (Encoding::Utf16Le, true);
        }
        
        if content.starts_with(&[0xFE, 0xFF]) {
            return (Encoding::Utf16Be, true);
        }
        
        // Try UTF-8 validation
        if std::str::from_utf8(content).is_ok() {
            return (Encoding::Utf8, false);
        }
        
        // Fallback to Latin1
        (Encoding::Latin1, false)
    }
    
    fn detect_newline(content: &[u8]) -> Newline {
        // Check for CRLF first
        for window in content.windows(2) {
            if window == b"\r\n" {
                return Newline::Crlf;
            }
        }
        
        // Default to LF
        Newline::Lf
    }
    
    /// Read file with encoding detection
    pub fn read_file(path: &Path) -> Result<(String, FileMetadata), std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        
        let metadata = Self::detect(&bytes);
        let text = Self::decode(&bytes, metadata.encoding)?;
        
        Ok((text, metadata))
    }
    
    /// Write file preserving encoding
    pub fn write_file(
        path: &Path,
        content: &str,
        metadata: &FileMetadata,
    ) -> Result<(), std::io::Error> {
        let bytes = Self::encode(content, metadata.encoding)?;
        
        let mut file = std::fs::File::create(path)?;
        file.write_all(&bytes)?;
        
        Ok(())
    }
    
    fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, std::io::Error> {
        match encoding {
            Encoding::Utf8 => {
                String::from_utf8(bytes.to_vec())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
            Encoding::Utf8Bom => {
                let content = &bytes[3..]; // Skip BOM
                String::from_utf8(content.to_vec())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
            Encoding::Utf16Le => {
                let content = &bytes[2..]; // Skip BOM
                let u16_vec: Vec<u16> = content
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&u16_vec)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
            Encoding::Utf16Be => {
                let content = &bytes[2..]; // Skip BOM
                let u16_vec: Vec<u16> = content
                    .chunks_exact(2)
                    .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&u16_vec)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
            Encoding::Latin1 => {
                // Proper Latin1 to UTF-8 conversion
                // Latin1 bytes 0-127 are ASCII, 128-255 map to Unicode U+0080-U+00FF
                Ok(bytes.iter().map(|&b| char::from_u32(b as u32).unwrap()).collect())
            }
        }
    }
    
    fn encode(text: &str, encoding: Encoding) -> Result<Vec<u8>, std::io::Error> {
        match encoding {
            Encoding::Utf8 => Ok(text.as_bytes().to_vec()),
            Encoding::Utf8Bom => {
                let mut bytes = vec![0xEF, 0xBB, 0xBF];
                bytes.extend_from_slice(text.as_bytes());
                Ok(bytes)
            }
            Encoding::Utf16Le => {
                let mut bytes = vec![0xFF, 0xFE]; // BOM
                for ch in text.encode_utf16() {
                    bytes.extend_from_slice(&ch.to_le_bytes());
                }
                Ok(bytes)
            }
            Encoding::Utf16Be => {
                let mut bytes = vec![0xFE, 0xFF]; // BOM
                for ch in text.encode_utf16() {
                    bytes.extend_from_slice(&ch.to_be_bytes());
                }
                Ok(bytes)
            }
            Encoding::Latin1 => {
                // Convert back to Latin1 (lossy)
                Ok(text
                    .chars()
                    .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
                    .collect())
            }
        }
    }
    
    /// Normalize newlines in text
    pub fn normalize_newlines(text: &str, style: Newline) -> String {
        match style {
            Newline::Lf => text.replace("\r\n", "\n"),
            Newline::Crlf => {
                let normalized = text.replace("\r\n", "\n");
                normalized.replace('\n', "\r\n")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn detects_utf8() {
        let content = "Hello, World!";
        let metadata = FileMetadata::detect(content.as_bytes());
        assert_eq!(metadata.encoding, Encoding::Utf8);
        assert!(!metadata.has_bom);
    }
    
    #[test]
    fn detects_utf8_bom() {
        let mut content = vec![0xEF, 0xBB, 0xBF];
        content.extend_from_slice(b"Hello");
        let metadata = FileMetadata::detect(&content);
        assert_eq!(metadata.encoding, Encoding::Utf8Bom);
        assert!(metadata.has_bom);
    }
    
    #[test]
    fn detects_crlf() {
        let content = b"Line1\r\nLine2\r\n";
        let metadata = FileMetadata::detect(content);
        assert_eq!(metadata.newline, Newline::Crlf);
    }
    
    #[test]
    fn detects_lf() {
        let content = b"Line1\nLine2\n";
        let metadata = FileMetadata::detect(content);
        assert_eq!(metadata.newline, Newline::Lf);
    }
    
    #[test]
    fn roundtrip_with_bom() {
        let temp = NamedTempFile::new().unwrap();
        let text = "Hello, World!";
        let metadata = FileMetadata {
            encoding: Encoding::Utf8Bom,
            newline: Newline::Lf,
            has_bom: true,
        };
        
        FileMetadata::write_file(temp.path(), text, &metadata).unwrap();
        let (read_text, read_metadata) = FileMetadata::read_file(temp.path()).unwrap();
        
        assert_eq!(read_text, text);
        assert_eq!(read_metadata.encoding, Encoding::Utf8Bom);
    }
}
