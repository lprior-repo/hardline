import os
import re

def find_violations():
    unwrap_re = re.compile(r'\b(unwrap|expect)\(')
    pub_fn_re = re.compile(r'^\s*pub fn\s+(\w+)\s*\(')
    
    for root, dirs, files in os.walk('crates'):
        for file in files:
            if file.endswith('.rs') and 'test' not in file and 'mock' not in file:
                path = os.path.join(root, file)
                with open(path, 'r', encoding='utf-8') as f:
                    lines = f.readlines()
                    
                in_pub_fn = False
                fn_name = ""
                returns_result = False
                
                for i, line in enumerate(lines):
                    # Check for unwrap/expect
                    if unwrap_re.search(line) and not line.strip().startswith('//'):
                        # rudimentary check to avoid tests
                        if '#[test]' not in "".join(lines[max(0, i-10):i]):
                            print(f"Violation (unwrap/expect): {path}:{i+1}: {line.strip()}")
                    
                    # Check public functions
                    m = pub_fn_re.search(line)
                    if m:
                        fn_name = m.group(1)
                        if '->' in line:
                            if 'Result<' not in line and 'ValidationResult<' not in line and 'UseCaseResult<' not in line:
                                print(f"Warning (no Result): {path}:{i+1}: {line.strip()}")
                        else:
                            # Might be multi-line or return ()
                            if not line.strip().endswith('{') and i + 1 < len(lines) and '->' in lines[i+1]:
                                if 'Result<' not in lines[i+1]:
                                    pass # print(f"Warning (no Result on next line): {path}:{i+1}: {fn_name}")
                            else:
                                print(f"Warning (no return type / Result): {path}:{i+1}: {line.strip()}")

if __name__ == '__main__':
    find_violations()
