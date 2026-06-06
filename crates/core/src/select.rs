//! S3 Select domain types (no AWS SDK types).

use crate::{Error, Result};

/// Object payload format for S3 Select input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectInputFormat {
    #[default]
    Csv,
    Json,
    Parquet,
}

/// Result row format for S3 Select output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectOutputFormat {
    #[default]
    Csv,
    Json,
}

/// Compression applied to the **stored object** (input decompression).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectCompression {
    #[default]
    None,
    Gzip,
    Bzip2,
}

/// CSV header handling for S3 Select input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectCsvFileHeaderInfo {
    #[default]
    None,
    Ignore,
    Use,
}

/// JSON input shape for S3 Select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectJsonInputType {
    #[default]
    Lines,
    Document,
}

/// CSV output quote behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectQuoteFields {
    Always,
    #[default]
    AsNeeded,
}

/// Supported CSV input serialization options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectCsvInputOptions {
    pub file_header_info: SelectCsvFileHeaderInfo,
    pub field_delimiter: Option<String>,
    pub quote_character: Option<String>,
    pub quote_escape_character: Option<String>,
    pub comments: Option<String>,
}

/// Supported CSV output serialization options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectCsvOutputOptions {
    pub field_delimiter: Option<String>,
    pub record_delimiter: Option<String>,
    pub quote_character: Option<String>,
    pub quote_escape_character: Option<String>,
    pub quote_fields: SelectQuoteFields,
}

/// Supported JSON input serialization options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectJsonInputOptions {
    pub input_type: SelectJsonInputType,
}

/// Supported JSON output serialization options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectJsonOutputOptions {
    pub record_delimiter: Option<String>,
}

/// ScanRange request body parameters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectScanRangeOptions {
    pub start: Option<i64>,
    pub end: Option<i64>,
}

/// SSE-C parameters for encrypted objects.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectSseCustomerOptions {
    pub algorithm: Option<String>,
    pub key: Option<String>,
    pub key_md5: Option<String>,
}

/// Options for running an S3 Select query on one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOptions {
    /// SQL expression (S3 Select / `s3object`).
    pub expression: String,
    pub input_format: SelectInputFormat,
    pub output_format: SelectOutputFormat,
    pub compression: SelectCompression,
    pub csv_input: SelectCsvInputOptions,
    pub csv_output: SelectCsvOutputOptions,
    pub json_input: SelectJsonInputOptions,
    pub json_output: SelectJsonOutputOptions,
    pub scan_range: SelectScanRangeOptions,
    pub sse_customer: SelectSseCustomerOptions,
}

impl SelectCsvInputOptions {
    pub fn validate(&self) -> Result<()> {
        validate_single_byte("CSV input field delimiter", self.field_delimiter.as_deref())?;
        validate_single_byte("CSV input quote character", self.quote_character.as_deref())?;
        validate_single_byte(
            "CSV input quote escape character",
            self.quote_escape_character.as_deref(),
        )?;
        validate_single_byte("CSV input comment character", self.comments.as_deref())?;

        Ok(())
    }
}

impl SelectCsvOutputOptions {
    pub fn validate(&self) -> Result<()> {
        validate_single_byte(
            "CSV output field delimiter",
            self.field_delimiter.as_deref(),
        )?;
        validate_record_delimiter(
            "CSV output record delimiter",
            self.record_delimiter.as_deref(),
        )?;
        validate_single_byte(
            "CSV output quote character",
            self.quote_character.as_deref(),
        )?;
        validate_single_byte(
            "CSV output quote escape character",
            self.quote_escape_character.as_deref(),
        )?;

        Ok(())
    }
}

impl SelectOptions {
    pub fn validate(&self) -> Result<()> {
        self.csv_input.validate()?;
        self.csv_output.validate()?;
        self.validate_scan_range()?;

        Ok(())
    }

    pub fn validate_scan_range(&self) -> Result<()> {
        let scan_range = &self.scan_range;
        if scan_range.start.is_none() && scan_range.end.is_none() {
            return Ok(());
        }

        if matches!(self.input_format, SelectInputFormat::Parquet) {
            return Err(Error::General(
                "ScanRange is not supported for Parquet input.".to_string(),
            ));
        }
        if matches!(self.input_format, SelectInputFormat::Json)
            && matches!(self.json_input.input_type, SelectJsonInputType::Document)
        {
            return Err(Error::General(
                "ScanRange is not supported for JSON document input.".to_string(),
            ));
        }
        if !matches!(self.compression, SelectCompression::None)
            && matches!(
                self.input_format,
                SelectInputFormat::Csv | SelectInputFormat::Json
            )
        {
            return Err(Error::General(
                "ScanRange is only supported for uncompressed CSV/JSON input.".to_string(),
            ));
        }
        if scan_range.start.is_some_and(|start| start < 0)
            || scan_range.end.is_some_and(|end| end < 0)
        {
            return Err(Error::General(
                "ScanRange start and end must be non-negative.".to_string(),
            ));
        }
        if let (Some(start), Some(end)) = (scan_range.start, scan_range.end)
            && start > end
        {
            return Err(Error::General(
                "ScanRange start must not be greater than end.".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for SelectOptions {
    fn default() -> Self {
        Self {
            expression: String::new(),
            input_format: SelectInputFormat::Csv,
            output_format: SelectOutputFormat::Csv,
            compression: SelectCompression::None,
            csv_input: SelectCsvInputOptions::default(),
            csv_output: SelectCsvOutputOptions::default(),
            json_input: SelectJsonInputOptions::default(),
            json_output: SelectJsonOutputOptions::default(),
            scan_range: SelectScanRangeOptions::default(),
            sse_customer: SelectSseCustomerOptions::default(),
        }
    }
}

fn validate_single_byte(name: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value
        && value.len() != 1
    {
        return Err(Error::General(format!("{name} must be exactly one byte.")));
    }
    Ok(())
}

fn validate_record_delimiter(name: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value
        && value.len() != 1
        && value != "\r\n"
    {
        return Err(Error::General(format!(
            "{name} must be exactly one byte or CRLF."
        )));
    }
    Ok(())
}
