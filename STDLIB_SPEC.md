# DGM Standard Library – Stable Modules (v0.2.0)

> Official stable modules for DGM Alpha_Major_1

---

## Module Index

| Module | Status | Primary Functions | Risk |
|--------|--------|-------------------|------|
| `math` | Stable | sqrt, sin, cos, tan, random, ceil, floor, abs, min, max, pow | Low |
| `io` | Stable | read_file, write_file, read_dir, mkdir | Low |
| `fs` | Stable | read, write, append, delete, list, exists | Low |
| `os` | Stable | exec, env, platform, sleep, get_env | Low |
| `json` | Stable | parse, stringify | Low |
| `http` | Stable | get, post, serve | Low |
| `crypto` | Stable | sha256, md5, base64_encode, base64_decode, hmac_sha256 | Low |
| `regex` | Stable | match, split, replace, find_all | Low |
| `net` | Stable | tcp_connect, tcp_listen | Low |
| `time` | Stable | now, timestamp, strftime, sleep | Low |
| `thread` | Stable | spawn, join | Low |
| `xml` | Stable | parse, stringify | Low |
| `security` | Internal | set_sandbox_path, is_sandboxed | Internal |

---

## Arithmetic Module (`math`)

**Stable functions**:
```
math.sqrt(n)              # Square root
math.sin(radians)         # Sine
math.cos(radians)         # Cosine
math.tan(radians)         # Tangent
math.abs(n)               # Absolute value
math.ceil(n)              # Round up
math.floor(n)             # Round down
math.round(n)             # Round to nearest
math.min(a, b)            # Minimum
math.max(a, b)            # Maximum
math.pow(base, exp)       # Power
math.random()             # Random [0, 1)
math.log(n)               # Natural log
math.log10(n)             # Log base 10
```

**Frozen**: No additions without major version.

---

## File I/O Module (`io`)

**Stable functions**:
```
io.read_file(path)        # Read entire file as string
io.write_file(path, data) # Write data to file
io.read_dir(path)         # List directory contents
io.mkdir(path)            # Create directory
```

**Constraints**: No sandbox restrictions on `io` (replaced by `fs`).

---

## Sandboxed Filesystem Module (`fs`)

**Stable functions**:
```
fs.read(path)             # Read file (sandboxed)
fs.write(path, data)      # Write file (sandboxed)
fs.append(path, data)     # Append to file (sandboxed)
fs.delete(path)           # Delete file (sandboxed)
fs.list(dir)              # List directory (sandboxed)
fs.exists(path)           # Check existence (sandboxed)
```

**Constraints**: Obeys security sandbox set by `security.set_sandbox_path()`.

---

## OS Module (`os`)

**Stable functions**:
```
os.exec(cmd, args)        # Execute shell command
os.get_env(name)          # Get environment variable
os.platform()             # Get OS (linux, windows, macos)
os.sleep(ms)              # Sleep milliseconds
```

**Constraints**: `exec` requires security approval.

---

## JSON Module (`json`)

**Stable functions**:
```
json.parse(string)        # Parse JSON string, return DgmValue
json.stringify(value)     # Convert DgmValue to JSON string
```

**Constraints**: 
- Optimized hot-path (zero-copy parsing when possible)
- Pooled Vec<u8> for encoding (no per-request String allocation)
- Lazy evaluation supported

---

## HTTP Module (`http`)

**Stable functions**:
```
http.get(url)                    # GET request
http.post(url, data)             # POST request
http.serve(port, handler)        # Start HTTP server
http.request(method, url, opts)  # Generic request
```

**Constraints**:
- Request model: `req.headers`, `req.query`, `req.params` are zero-alloc views
- `req.json()` is lazy-loaded
- RequestShell is optimized representation
- No hidden allocations in hot path

---

## Cryptography Module (`crypto`)

**Stable functions**:
```
crypto.sha256(data)          # SHA256 hash
crypto.md5(data)             # MD5 hash
crypto.base64_encode(data)   # Base64 encode
crypto.base64_decode(data)   # Base64 decode
crypto.hmac_sha256(key, msg) # HMAC-SHA256
```

**Status**: Stable for general use.

---

## Regular Expressions Module (`regex`)

**Stable functions**:
```
regex.match(pattern, text)        # Check if pattern matches
regex.split(pattern, text)        # Split by pattern
regex.replace(pattern, text, rep) # Replace matches
regex.find_all(pattern, text)     # Find all matches
```

**Status**: Stable, uses standard `regex` crate.

---

## Network Module (`net`)

**Stable functions**:
```
net.tcp_connect(host, port)       # Connect to TCP socket
net.tcp_listen(port)              # Listen on TCP port
```

**Status**: Stable, low-level TCP only.

---

## Time Module (`time`)

**Stable functions**:
```
time.now()                   # Current UNIX timestamp (seconds)
time.timestamp()             # Alias for now()
time.strftime(fmt, ts)       # Format timestamp
time.sleep(ms)               # Sleep milliseconds
```

**Status**: Stable for general use.

---

## Threading Module (`thread`)

**Stable functions**:
```
thread.spawn(func)           # Spawn thread, return handle
thread.join(handle)          # Wait for thread completion
```

**Constraints**: 
- Concurrency system is stable
- Environment lifecycle is bounded (fixed Rc cycles)
- request_scope reports 0 survivors

---

## XML Module (`xml`)

**Stable functions**:
```
xml.parse(string)            # Parse XML, return tree
xml.stringify(tree)          # Convert tree to XML string
```

**Status**: Stable, uses `quick-xml` crate.

---

## Security Module (`security`)

**Internal status**: Not for general use.

**Functions**:
```
security.set_sandbox_path(path)  # Define sandbox boundary
security.is_sandboxed()          # Check if sandboxing enabled
```

**Constraints**: Thread-local configuration, affects `fs` module only.

---

## STABILITY GUARANTEES

1. **No API removals** in 0.2.x series
2. **Function signatures frozen** (parameter count, order)
3. **Return types stable** (will not change JSON shape)
4. **Error format consistent** (follows `[ERROR]` format from spec)
5. **Memory behavior predictable** (no hidden allocations in stable modules)

---

## MIGRATION PATH

If breaking changes needed **→** Major version bump (0.3.0)

Current status: **No breaking changes planned for v0.2.x**

---

**Last Updated**: DGM v0.2.0  
**Status**: All modules stable ✓
