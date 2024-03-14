# Ruslox Error Codes
Ruslox uses `codespan-reporting` to emit pretty error diagnostics with error codes attached. Error codes can help we programmers find the location of error occurrences quickly.

## Compile Error Codes
- `E0001`: too many constants in one chunk
- `E0002`: unexpected character
- `E0003`: uninterpretable number literal
- `E0004`: unterminated string
- `E0005`: unrecognized statement
- `E0006`: missing specific token
- `E0007`: missing variable name
- `E0008`: invalid assignment target

## Runtime Error Codes
- `E1001`: stack overflow
- `E1002`: stack underflow
- `E1003`: operands must be numbers
- `E1004`: operand must be number
- `E1005`: concatenation operands must be both numbers or both strings.
- `E1006`: invalid name of global definition
- `E1007`: defining global with empty stack
- `E1008`: undefined global
- `E1009`: get local with empty stack slot
- `E1010`: set local with empty stack slot
- `E1011`: jumping out of code
- `E1012`: jump condition required but stack is empty
- `E1013`: loop back out of code