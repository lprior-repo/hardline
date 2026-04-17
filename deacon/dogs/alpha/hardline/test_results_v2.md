### 1. Acquire a lock
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock acquire session-1 --agent agent-A`
**Exit Code:** 0
**Stdout:**
```
Lock acquired: session-1 for agent agent-A (expires: 2026-03-30 04:35:00.562477220 UTC)
```
**Stderr:**
```
```

### 2. Check status
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock status session-1`
**Exit Code:** 0
**Stdout:**
```
Locked: session session-1 held by agent-A (expires: 2026-03-30 04:35:00.562477220 UTC)
```
**Stderr:**
```
```

### 3. List locks
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock list`
**Exit Code:** 0
**Stdout:**
```
SESSION              AGENT                EXPIRES                  
-----------------------------------------------------------------
session-1            agent-A              2026-03-30 04:35:00.562477220 UTC
```
**Stderr:**
```
```

### 4. Send heartbeat
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock heartbeat session-1 --agent agent-A`
**Exit Code:** 0
**Stdout:**
```
Heartbeat sent: session-1 (new expiration: 2026-03-30 04:35:00.587851495 UTC)
```
**Stderr:**
```
```

### 5. Test conflict
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock acquire session-1 --agent agent-B`
**Exit Code:** 90
**Stdout:**
```
```
**Stderr:**
```
Error: Session 'session-1' is locked by 'agent-A'
```

### 6. Release lock
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock release session-1 --agent agent-A`
**Exit Code:** 0
**Stdout:**
```
Lock released: session-1
```
**Stderr:**
```
```

### 7. Verify released status
**Command:** `/home/lewis/src/hl-kb8/target/debug/scp-cli lock status session-1`
**Exit Code:** 0
**Stdout:**
```
Unlocked: session session-1
```
**Stderr:**
```
```

