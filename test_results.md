### 1. Acquire a lock
**Command:** `./target/debug/scp-cli lock acquire session-1 --agent agent-A`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### 2. Check status
**Command:** `./target/debug/scp-cli lock status session-1`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### 3. List locks
**Command:** `./target/debug/scp-cli lock list`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### 4. Send heartbeat
**Command:** `./target/debug/scp-cli lock heartbeat session-1 --agent agent-A`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### 5. Test conflict
**Command:** `./target/debug/scp-cli lock acquire session-1 --agent agent-B`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### 6. Release lock
**Command:** `./target/debug/scp-cli lock release session-1 --agent agent-A`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

### 7. Verify released status
**Command:** `./target/debug/scp-cli lock status session-1`
**Exit Code:** 2
**Stdout:**
```
```
**Stderr:**
```
error: unrecognized subcommand 'lock'

Usage: scp-cli [OPTIONS] <COMMAND>

For more information, try '--help'.
```

