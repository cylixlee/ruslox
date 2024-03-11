# Ruslox
Ruslox is the Rust implementation of Lox language, with some difference from the `clox` compiler and VM.

- [Ruslox](#ruslox)
  - [Error Codes](#error-codes)
    - [Compile Error Codes](#compile-error-codes)
    - [Runtime Error Codes](#runtime-error-codes)


## Error Codes
The error code concept is introduced with `codespan-reporting` as the supporting crate. Instead of simply `printf` the error message and line number into the `stdout` / `stderr` stream in `clox`, Ruslox generates diagnostics with messages, notes, and labels pointing the position of compile errors in source. Error codes can help locating the potential internal problems in the meantime.

### Compile Error Codes
- `E0001`: too many constants in one chunk
- `E0002`: unexpected character
- `E0003`: uninterpretable number literal
- `E0004`: unterminated string

### Runtime Error Codes
- `E1001`: stack overflow
- `E1002`: stack underflow
- `E1003`: operands must be numbers
- `E1004`: operand must be number
- `E1005`: concatenation operands must be both numbers or both strings.