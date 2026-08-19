from pathlib import Path
p=Path(r'C:\Hijosdelsol\VelvetEngine\crates\velvet-cli\src\main.rs')
lines=p.read_text(encoding='utf-8').splitlines()
for i,line in enumerate(lines):
    if 'Capture' in line or 'cmd_app_capture' in line:
        start=max(0,i-8); end=min(len(lines),i+18)
        print(f'--- {i+1} ---')
        for j in range(start,end):
            print(f'{j+1}: {lines[j]}')
