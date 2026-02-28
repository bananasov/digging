use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiResponse<'a, T> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,

    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError<'a>>,

    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub message: Option<&'a str>,
}

/// A struct with nothing, used as a default placeholder
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct None {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResponseMeta {
    pub limit: i32,
    pub total: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiError<'a> {
    pub code: &'a str,
    pub message: &'a str,
    pub details: &'a [ErrorDetail<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorDetail<'a> {
    pub field: &'a str,
    pub message: &'a str,
}

impl<T: Serialize> Default for ApiResponse<'_, T> {
    fn default() -> Self {
        Self {
            data: None,
            meta: None,
            error: None,
            message: None,
        }
    }
}

impl<'a, T: Serialize> ApiResponse<'a, T> {
    /// Create a success response with data
    pub const fn success(data: T) -> Self {
        Self {
            data: Some(data),
            meta: None,
            error: None,
            message: None,
        }
    }

    /// Create a success response with data and a message
    pub const fn success_with_message(data: T, message: &'a str) -> Self {
        Self {
            data: Some(data),
            meta: None,
            error: None,
            message: Some(message),
        }
    }

    /// Create a success response with data and metadata
    pub const fn success_with_meta(data: T, meta: ResponseMeta) -> Self {
        Self {
            data: Some(data),
            meta: Some(meta),
            error: None,
            message: None,
        }
    }

    /// Create an error response
    #[must_use]
    pub const fn error(error: ApiError<'a>) -> Self {
        Self {
            data: None,
            meta: None,
            error: Some(error),
            message: None,
        }
    }

    /// Create an error response with a message
    #[must_use]
    pub const fn error_with_message(error: ApiError<'a>, message: &'a str) -> Self {
        Self {
            data: None,
            meta: None,
            error: Some(error),
            message: Some(message),
        }
    }
}

impl<'a> ApiError<'a> {
    /// Create a simple error with code and message
    #[must_use]
    pub const fn new(code: &'a str, message: &'a str) -> Self {
        Self {
            code,
            message,
            details: &[],
        }
    }

    /// Create an error with details
    #[must_use]
    pub const fn with_details(
        code: &'a str,
        message: &'a str,
        details: &'a [ErrorDetail<'a>],
    ) -> Self {
        Self {
            code,
            message,
            details,
        }
    }
}

impl<'a> ErrorDetail<'a> {
    /// Create a new error detail
    #[must_use]
    pub const fn new(field: &'a str, message: &'a str) -> Self {
        Self { field, message }
    }
}
