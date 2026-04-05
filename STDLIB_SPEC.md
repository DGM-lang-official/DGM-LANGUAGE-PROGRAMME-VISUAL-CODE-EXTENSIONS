# DGM Standard Library – Stable Modules (v0.2.0)

> Official stable modules for DGM Alpha_Major_1

---

## Module Index

| Module | Status | Primary Functions | Risk |
|--------|--------|-------------------|------|
| `math` | Stable | sqrt, sin, cos, tan, random, ceil, floor, abs, min, max, pow | Low |
| `io` | Stable | read_file, write_file, list_dir, exists | Low |
| `fs` | Stable | read, write, append, delete, list, exists | Low |
| `os` | Stable | exec, spawn, run, run_timeout, env | Low |
| `json` | Stable | parse, stringify, raw_parts | Low |
| `http` | Stable | get, post, serve | Low |
| `crypto` | Stable | sha256, md5, base64_encode, base64_decode, hmac_sha256 | Low |
| `regex` | Stable | match, split, replace, find_all | Low |
| `net` | Stable | tcp_connect, tcp_listen | Low |
| `time` | Stable | now, now_ms, format, parse, elapsed | Low |
| `thread` | Stable | sleep, available_cpus | Low |
| `xml` | Stable | parse, stringify, query | Low |
| `security` | Internal | configure, status | Internal |

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
io.append_file(path, data)# Append text to file
io.exists(path)           # Check path existence
io.delete(path)           # Delete file or directory
io.mkdir(path)            # Create directory
io.list_dir(path)         # List directory contents
io.cwd()                  # Current working directory
io.input(prompt?)         # Read line from stdin
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

**Constraints**: Obeys security policy configured via `security.configure(...)`.

---

## OS Module (`os`)

**Stable functions**:
```
os.exec(cmd)              # Execute shell command through system shell
os.spawn(cmd)             # Spawn shell command, return handle + pid
os.run(program, args)     # Execute program without shell
os.run_timeout(program, args, timeout_ms) # Execute with timeout
os.wait(handle, timeout_ms?) # Wait on spawned process
os.env(name)              # Get environment variable
os.set_env(name, value)   # Set environment variable
os.platform()             # Get OS (linux, windows, macos)
os.sleep(ms)              # Sleep milliseconds
os.cwd()                  # Current working directory
os.chdir(path)            # Change working directory
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
time.now()                # Current UNIX timestamp (seconds)
time.now_ms()             # Current UNIX timestamp (milliseconds)
time.format(ts, fmt)      # Format UNIX timestamp
time.parse(str, fmt)      # Parse datetime string to UNIX timestamp
time.elapsed(start_ms)    # Milliseconds elapsed since start
```

**Status**: Stable for general use.

---

## Threading Module (`thread`)

**Stable functions**:
```
thread.sleep(ms)             # Sleep current thread
thread.available_cpus()      # Number of available CPUs
```

**Constraints**: 
- Utility helpers only in current runtime
- No user-exposed thread spawning API in v0.2.0 runtime

---

## XML Module (`xml`)

**Stable functions**:
```
xml.parse(string)            # Parse XML, return tree
xml.stringify(tree)          # Convert tree to XML string
xml.query(tree, path)        # Find first child node by dotted path
```

**Status**: Stable, uses `quick-xml` crate.

---

## Security Module (`security`)

**Internal status**: Not for general use.

**Functions**:
```
security.configure(opts_map)     # Update runtime security policy
security.status()                # Read current security policy
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
