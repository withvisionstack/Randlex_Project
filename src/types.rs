#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Str, Bool,
}

impl Type {
    pub fn from_name(name: &str) -> Option<Type> {
        Some(match name {
            "int8" => Type::Int8,     "int16" => Type::Int16,
            "int32" => Type::Int32,   "int64" => Type::Int64,
            "uint8" => Type::UInt8,   "uint16" => Type::UInt16,
            "uint32" => Type::UInt32, "uint64" => Type::UInt64,
            "float32" => Type::Float32, "float64" => Type::Float64,
            "str" => Type::Str,       "bool" => Type::Bool,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Type::Int8 => "int8",     Type::Int16 => "int16",
            Type::Int32 => "int32",   Type::Int64 => "int64",
            Type::UInt8 => "uint8",   Type::UInt16 => "uint16",
            Type::UInt32 => "uint32", Type::UInt64 => "uint64",
            Type::Float32 => "float32", Type::Float64 => "float64",
            Type::Str => "str",       Type::Bool => "bool",
        }
    }

    pub fn int_range(&self) -> Option<(i128, i128)> {
        Some(match self {
            Type::Int8 => (i8::MIN as i128, i8::MAX as i128),
            Type::Int16 => (i16::MIN as i128, i16::MAX as i128),
            Type::Int32 => (i32::MIN as i128, i32::MAX as i128),
            Type::Int64 => (i64::MIN as i128, i64::MAX as i128),
            Type::UInt8 => (0, u8::MAX as i128),
            Type::UInt16 => (0, u16::MAX as i128),
            Type::UInt32 => (0, u32::MAX as i128),
            Type::UInt64 => (0, u64::MAX as i128),
            _ => return None,
        })
    }

    pub fn is_integer(&self) -> bool { self.int_range().is_some() }
    pub fn is_float(&self) -> bool { matches!(self, Type::Float32 | Type::Float64) }
    pub fn is_numeric(&self) -> bool { self.is_integer() || self.is_float() }
}