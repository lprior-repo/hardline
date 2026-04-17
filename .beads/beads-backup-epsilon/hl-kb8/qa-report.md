# QA Report for Bead hl-kb8: CLI Lock Integration

## Test Results

1. **Acquire a lock**: `scp-cli lock acquire session-1 --agent agent-A`
   - **Exit Code**: 0
   - **Actual Output**: `Lock acquired: session-1 for agent agent-A (expires: 2026-03-30 04:35:00.562477220 UTC)`
   - **Status**: PASS

2. **Check status (Locked)**: `scp-cli lock status session-1`
   - **Exit Code**: 0
   - **Actual Output**: `Locked: session session-1 held by agent-A (expires: 2026-03-30 04:35:00.562477220 UTC)`
   - **Status**: PASS

3. **List locks**: `scp-cli lock list`
   - **Exit Code**: 0
   - **Actual Output**: 
     ```
     SESSION              AGENT                EXPIRES                  
     -----------------------------------------------------------------
     session-1            agent-A              2026-03-30 04:35:00.562477220 UTC
     ```
   - **Status**: PASS

4. **Send heartbeat**: `scp-cli lock heartbeat session-1 --agent agent-A`
   - **Exit Code**: 0
   - **Actual Output**: `Heartbeat sent: session-1 (new expiration: 2026-03-30 04:35:00.587851495 UTC)`
   - **Status**: PASS

5. **Test conflict**: `scp-cli lock acquire session-1 --agent agent-B`
   - **Exit Code**: 90
   - **Actual Stderr**: `Error: Session "session-1" is locked by "agent-A"`
   - **Status**: PASS (Behaviorally correct; exit code matches ValidationError)

6. **Release lock**: `scp-cli lock release session-1 --agent agent-A`
   - **Exit Code**: 0
   - **Actual Output**: `Lock released: session-1`
   - **Status**: PASS

7. **Verify released status**: `scp-cli lock status session-1`
   - **Exit Code**: 0
   - **Actual Output**: `Unlocked: session session-1`
   - **Status**: PASS
