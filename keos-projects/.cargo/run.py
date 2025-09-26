#!/usr/bin/env python3
import json
import hashlib
import subprocess
import urllib.request
import tarfile
import os
import sys
import subprocess
import shutil
import signal
import sys
import json
import string
from pathlib import Path

script_dir = os.path.dirname(os.path.realpath(__file__))
kernel_path = os.path.realpath(sys.argv[1])
output_dir = os.path.dirname(sys.argv[1])
grub_files_dir = os.path.join(output_dir, 'target/grub_files')
current_path = os.path.realpath('.')

try:
    cpuinfo = subprocess.check_output(['cat', '/proc/cpuinfo']).decode()
    if ('vmx' in cpuinfo or 'svm' in cpuinfo) and 'QEMU_DISABLE_KVM' not in os.environ:
        QEMU_CPU_TYPE = f"-cpu host{os.getenv('QEMU_CPU_OPT', '')} -enable-kvm"
    else:
        QEMU_CPU_TYPE = f"-M q35 -cpu qemu64{os.getenv('QEMU_CPU_OPT', '')},x2apic"
except Exception as e:
    eprint('Error detecting CPU features:', e)
    QEMU_CPU_TYPE = '-cpu qemu64'

def strip_unprintable(s):
    return ''.join(c for c in s if c in string.printable)

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

def find_qemu_pids():
    matching_pids = []
    
    # Iterate over all directories in /proc. PIDs are represented by numbers.
    for pid_dir in os.listdir('/proc'):
        if pid_dir.isdigit():
            try:
                # Construct the path to the command line file
                cmdline_path = os.path.join('/proc', pid_dir, 'cmdline')
                
                # Check if the command line file exists and is readable
                if os.path.exists(cmdline_path) and os.access(cmdline_path, os.R_OK):
                    with open(cmdline_path, 'rb') as f:
                        # Command line arguments are null-byte separated
                        cmdline_bytes = f.read()
                        cmdline_str = cmdline_bytes.replace(b'\x00', b' ').decode('utf-8').strip()
                        
                        # Check if the command line contains both required strings
                        if "qemu-system-x86_64" in cmdline_str and "kernel.iso" in cmdline_str:
                            matching_pids.append(int(pid_dir))
            except (IOError, OSError, UnicodeDecodeError):
                # Handles cases where a process disappears, or we can't read the file
                continue

    return matching_pids

def is_ignored(file_path):
    IGNORED_PATTERNS = [
        'target/', 'build/', 'build_objects/', 'keos_kernel', 'ffs.bin', 'sfs.bin', '.o', 'kelibc.a',
        '.cargo/template/', 'Cargo.lock',
    ]
    
    # Special handling for rootfs - ignore rootfs/ but keep specific files
    ROOTFS_EXCEPTIONS = ['hello', 'os-release', 'simple_fs.tar']
    
    path_str = str(file_path)
    
    # Check if it's in rootfs directory
    if '/rootfs/' in path_str or path_str.endswith('/rootfs'):
        # If it's a rootfs directory itself, ignore it
        if path_str.endswith('/rootfs'):
            return True
        
        # If it's a file in rootfs, check if it's an exception
        file_name = Path(path_str).name
        for exception in ROOTFS_EXCEPTIONS:
            if exception == 'hello' and file_name.startswith('hello'):
                return False
            elif exception == file_name:
                return False
        
        # If not an exception, ignore it
        return True
    
    # Check other ignored patterns
    for pattern in IGNORED_PATTERNS:
        if pattern.endswith('/'):
            if f'/{pattern[:-1]}/' in path_str or path_str.endswith(f'/{pattern[:-1]}'):
                return True
        elif '*' in pattern:
            import fnmatch
            if fnmatch.fnmatch(Path(path_str).name, pattern):
                return True
        else:
            if pattern in path_str:
                return True
    return False

def get_file_hash(file_path):
    '''Calculate SHA256 hash of a file'''
    try:
        with open(file_path, 'rb') as f:
            return hashlib.sha256(f.read()).hexdigest()
    except (FileNotFoundError, PermissionError):
        return None

def get_project_whitelist(project_num):
    '''Get whitelist for a specific project only (not cumulative)'''
    current_dir = Path.cwd()  # Should be in grader/
    keos_projects_dir = current_dir.parent.parent  # Go to keos-projects/ (grandparent)
    project_dir = keos_projects_dir / f'keos-project{project_num}'
    grade_target_file = project_dir / 'grader' / '.grade-target'

    if not grade_target_file.exists():
        return []

    try:
        with open(grade_target_file, 'r') as f:
            config = json.load(f)
            return config.get('whitelist', [])
    except (json.JSONDecodeError, FileNotFoundError):
        return []

def get_repository_root():
    '''Get the repository root directory from current grader location'''
    current_dir = Path.cwd()  # Should be in grader/
    project_dir = current_dir.parent  # Go to keos-projectN/
    keos_projects_dir = project_dir.parent  # Go to keos-projects/
    repo_root = keos_projects_dir.parent  # Go to repository root
    return repo_root

def build_cumulative_whitelist(current_project_num):
    '''Build cumulative whitelist by importing all previous projects' whitelists'''
    cumulative_whitelist = set()
    project_details = {}  # Store details for printing

    for project_num in range(1, current_project_num + 1):
        project_whitelist = get_project_whitelist(project_num)
        if project_whitelist:
            project_details[project_num] = project_whitelist
            cumulative_whitelist.update(project_whitelist)

    return list(cumulative_whitelist), project_details

def check_directory_integrity(directory_name, cumulative_whitelist, current_project_num, mode='warn'):
    '''Check integrity of a specific directory - only report violations'''
    repo_root = get_repository_root()
    check_dir = repo_root / directory_name
    original_dir = Path('../../.cargo/template').resolve()

    if not check_dir.exists():
        return []

    violations = []

    # Collect all relevant files in the directory
    for root, dirs, files in os.walk(check_dir):
        # Filter directories based on current project scope
        if directory_name == 'keos-projects' and current_project_num > 0:
            # Only check project directories up to current project number
            filtered_dirs = []
            for d in dirs:
                if is_ignored(Path(root) / d):
                    continue
                if d.startswith('keos-project'):
                    try:
                        project_num = int(d.replace('keos-project', ''))
                        if project_num <= current_project_num:
                            filtered_dirs.append(d)
                    except ValueError:
                        # If we can't parse the number, skip it
                        pass
                else:
                    filtered_dirs.append(d)
            dirs[:] = filtered_dirs
        else:
            dirs[:] = [d for d in dirs if not is_ignored(Path(root) / d)]

        for file in files:
            file_path = Path(root) / file
            if not is_ignored(file_path):
                rel_path = file_path.relative_to(check_dir)

                # Process whitelist based on directory type
                is_whitelisted = False
                if directory_name == 'keos-projects':
                    # Check if file is in any project and whitelisted
                    project_rel_path = str(rel_path)
                    # Check if this file matches any project pattern up to current project
                    for project_num in range(1, current_project_num + 1):  # Check projects 1 to current
                        project_prefix = f'keos-project{project_num}/'
                        if project_rel_path.startswith(project_prefix):
                            file_in_project = project_rel_path.replace(project_prefix, '')
                            if file_in_project in cumulative_whitelist:
                                is_whitelisted = True
                                break

                # Skip if whitelisted
                if is_whitelisted:
                    continue

                # Find original file
                original_file = original_dir / directory_name / rel_path

                if not original_file.exists():
                    continue

                # Compare file hashes for existing files
                current_hash = get_file_hash(file_path)
                original_hash = get_file_hash(original_file)

                if current_hash != original_hash:
                    violations.append({
                        'file_path': file_path,
                        'rel_path': str(rel_path),
                        'original_file': original_file,
                    })

    # Handle violations
    if violations:
        print(f'\n{directory_name}/ - {len(violations)} violation(s):')

        for violation in violations:
            rel_path = violation['rel_path']

            if mode == 'warn':
                print(f'  {rel_path} (MODIFIED)')
            elif mode == 'override':
                try:
                    import shutil
                    shutil.copy2(violation['original_file'], violation['file_path'])
                    print(f'  RESTORED: {rel_path} (restored)')
                except Exception as e:
                    print(f'  ERROR: {rel_path} (restore failed: {e})')
                    print('ERROR: File restoration failed. Aborting.')
                    sys.exit(1)

    return [v['rel_path'] for v in violations]

def ensure_template_freshness():
    '''Ensure the template repository is in a clean, up-to-date state'''
    template_dir = Path('../../.cargo/template').resolve()
    
    if not template_dir.exists():
        print('ERROR: Template directory does not exist')
        sys.exit(1)
    
    if not (template_dir / '.git').exists():
        print('ERROR: Template directory is not a git repository')
        sys.exit(1)
    
    try:
        # Change to template directory
        original_cwd = os.getcwd()
        os.chdir(template_dir)
        
        # Reset to clean state (discard any local changes)
        subprocess.run(['git', 'reset', '--hard', 'HEAD'], 
                      check=True, capture_output=True)
        
        # Clean untracked files
        subprocess.run(['git', 'clean', '-fd'], 
                      check=True, capture_output=True)
        
        # Fetch latest changes
        subprocess.run(['git', 'fetch', 'origin'], 
                      check=True, capture_output=True)
        
        # Reset to latest origin/main
        subprocess.run(['git', 'reset', '--hard', 'origin/main'], 
                      check=True, capture_output=True)
        
        # Return to original directory
        os.chdir(original_cwd)
        
    except subprocess.CalledProcessError as e:
        print(f'ERROR: Failed to update template repository: {e}')
        sys.exit(1)
    except Exception as e:
        print(f'ERROR: Unexpected error updating template: {e}')
        sys.exit(1)

def check_three_directory_integrity(cumulative_whitelist, current_project_num, mode='warn'):
    '''Check integrity of the three main directories - only report violations'''
    # The three main directories that should be checked for integrity
    CHECKED_DIRECTORIES = ['fs', 'keos', 'keos-projects']

    # Ensure template repository is fresh and up-to-date
    ensure_template_freshness()

    all_violations = {}

    # Check each of the three main directories
    for directory in CHECKED_DIRECTORIES:
        if directory == 'keos-projects':
            violations = check_directory_integrity(directory, cumulative_whitelist, current_project_num, mode)
        else:
            violations = check_directory_integrity(directory, [], 0, mode)  # Use 0 for non-keos-projects

        if violations:
            all_violations[directory] = violations

    return all_violations

def check_whitelist_compliance(whitelist):
    '''Check whitelist compliance'''
    mode = os.environ.get('KEOS_FILE_CHECK', 'warn')

    current_dir = Path.cwd()
    project_dir = current_dir.parent
    project_name = project_dir.name

    if not project_name.startswith('keos-project'):
        print(f'ERROR: Invalid project directory: {project_name}')
        sys.exit(1)

    try:
        project_num = int(project_name.split('project')[1])
    except (IndexError, ValueError):
        print(f'ERROR: Cannot parse project number from {project_name}')
        sys.exit(1)


    # Build cumulative whitelist with details
    cumulative_whitelist, project_details = build_cumulative_whitelist(project_num)

    # Print detailed whitelist by project
    if project_details:
        print(f'Whitelisted Files:')
        for proj_num in sorted(project_details.keys()):
            files = project_details[proj_num]
            print(f'  Project {proj_num}:')
            for file in sorted(files):
                print(f'    - {file}')
    # Check three-directory integrity
    all_violations = check_three_directory_integrity(cumulative_whitelist, project_num, mode)

    # Summary
    total_violations = sum(len(files) for files in all_violations.values())

    if total_violations != 0:
        print('----------------')
        if mode == 'warn':
            print(f'\nTotal {total_violations} violation(s) found:')
            print('   These modifications will be ignored during official grading')
            input('[*] Type enter to continue:')
        else:
            print(f'\n{total_violations} file(s) restored to original versions')


def setup_kernel_arguments(kernel_arg):
    grub_dir = os.path.join(output_dir, 'target/grub_files/boot/grub')
    os.makedirs(grub_dir, exist_ok=True)
    grub_cfg_path = os.path.join(grub_dir, 'grub.cfg')
    with open(grub_cfg_path, 'w') as grub_cfg:
        grub_cfg.write(f'''set default='0'
set timeout_style=hidden
set timeout=0

menuentry 'keos' {{
    multiboot2 /boot/keos {kernel_arg}
    boot
}}
''')

    subprocess.run([
        'grub-mkrescue', '/usr/lib/grub/i386-pc',
        '-o', os.path.join(output_dir, 'target/kernel.iso'),
        os.path.join(output_dir, 'target/grub_files')
    ], stderr=subprocess.DEVNULL)

def make_run_command(pj):
    return [
        'qemu-system-x86_64', '-nographic', '--boot', 'd',
        '-cdrom', os.path.join(output_dir, 'target/kernel.iso'),
        '-device', 'virtio-blk-pci,drive=kernel',
        '-drive', 'format=raw,if=none,file=keos_kernel,id=kernel,cache=none,readonly=on',
        '-device', f'virtio-blk-pci,drive=sfs',
        '-drive', f'format=raw,if=none,file=sfs.bin,id=sfs,cache=none',
        *([
            '-device', f'virtio-blk-pci,drive=ffs',
            '-drive', f'format=raw,if=none,file=ffs.bin,id=ffs,cache=none',
        ] if pj == 5 else []),
        *QEMU_CPU_TYPE.split(),
        '-serial', 'mon:stdio', '-no-reboot'
    ]

def run(pj, run_command):
    cpu = os.environ.get('MP', '4')
    mem = os.environ.get('MEM', '1G')

    run_command = [
        *run_command,
        '-smp', f'{cpu}',
        '-m', mem,
        '-s'
    ]
    setup_kernel_arguments(' '.join(sys.argv[2:]) if sys.argv[2:] else '')
    os.execvp(run_command[0], run_command)

def grade_single(run_command, target, cpu, mem, timeout, live):
    setup_kernel_arguments('--quite {}'.format(target))
    try:
        if live:
            p = subprocess.Popen(
                [
                    *run_command,
                    '-smp', f'{cpu}',
                    '-m', mem
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE
            )

            output = b''
            while p.poll() == None:
                out = p.stdout.readline()
                if b'\x1bc' in out:
                    output = b''
                    continue
                sys.stdout.buffer.write(out)
                sys.stdout.flush()
                output += out
            print('---------')

            return output.decode('utf-8')
        else:
            return subprocess.run(
                [
                    *run_command,
                    '-smp', f'{cpu}',
                    '-m', mem
                ],
                stdin=subprocess.DEVNULL,
                capture_output=True,
                text=True,
                timeout=float(timeout)
            ).stdout
    except Exception as e:
        print(e)
        return 'timeout'


def grade(pj, run_command, rubrics, filt = None):
    if filt is None:
        print(f'Start Grading keos-project{pj}:')

    grading_result = {}
    tpassed = 0
    for rname, rubric in rubrics.items():
        grading_result[rname] = dict()
        for name, conf in rubric['tests'].items():
            timeout = conf.get('timeout', 30)
            mem = conf.get('mem', '512M')
            cpu = conf.get('cpu', 4)

            if filt is not None and name not in filt and filt != []:
                continue

            if filt is None:
                print(f'Running test: {name} ... ', end='', flush=True)

            output = grade_single(run_command, name, cpu, mem, timeout, filt is not None)
            passed = 'test result: ok. 1 passed; 0 failed' in output
            if filt is not None and 'panic' in output:
                return

            if passed and 'post-hook' in conf.keys() and type(conf['post-hook']) == type([]):
                try:
                    posthook_result = subprocess.run(
                        conf['post-hook'],
                        text=True,
                        stdin=subprocess.DEVNULL,
                        capture_output=True,
                        timeout=float(timeout)
                    )
                    passed = posthook_result.returncode == 0
                    if not passed:
                        output += '\nPOST-HOOK [`{}`] Failed: {}'.format(
                            ' '.join(conf['post-hook']),
                            posthook_result.stderr
                        )
                except:
                    passed = False

            tpassed += passed
            if filt is None:
                print('ok' if passed else 'fail')
            grading_result[rname][name] = (passed, output)

    return grading_result

def summarize(pj, rubrics, grading_result):
    print('----------------')
    print(f'Test Summary of `keos-project{pj}`:')
    failed = {}
    max_score = 0
    score = 0.0
    tests = 0
    passed = 0
    for rname, rubric in grading_result.items():
        rmaxscore = rubrics[rname]['score']
        rscore = 0
        max_score += rmaxscore
        o = []
        for name, (result, output) in rubric.items():
            o.append('- {}: {}'.format('pass' if result else 'fail', name))
            rscore += result
            tests += 1
            passed += result
            if not result:
                failed[name] = output
        this_score = rscore / float(len(rubric.items())) * rmaxscore
        score += this_score
        o = '\n'.join(o)
        print(f'Rubric `{rname}` [{this_score:.2f}/{rmaxscore:.2f}]:\n{o}')
        print('----------------')

    print(f'TEST SUMMARY: {passed} of {tests} passed')
    print('TOTAL TESTING SCORE: {:.2f} / {:.2f}'.format(score, max_score))
    if passed == tests:
        print('🎉 ALL TEST PASSED -- PERFECT SCORE')
    if len(failed) > 0:
        print('----------------')
        for test, output in failed.items():
            print('TEST `{}`: {}'.format(test, strip_unprintable(output)))

def setup_gdb(run_command):
    with open('.gdbinit', 'w') as gdbinit:
        gdbinit.write('target remote 0:1234\n')
        gdbinit.write('symbol-file keos_kernel\n')
        gdbinit.write('set print frame-arguments all\n')

    run_command.append('-S')

    print('\n\033[46m\033[1;93mType \'gdb\' in this directory in separate terminal to start debugging session!\033[0m\n')
    return run_command

def usage():
    eprint('Usage: cargo [run|grade] [test-arg]')
    sys.exit(1)

def main():
    if len(sys.argv) < 2:
        usage()

    # Check is in grader dir
    if 'grader' not in current_path[-8:]:
        eprint("\n\033[46m\033[1;93mPlease run a grader on each grader's own directory.\033[0m\n")
        eprint('e.g. keos-project1/grader (O)')
        eprint('     keos-project1/grader/src (X)')
        eprint('     keos-project1 (X)\n')
        sys.exit(1)
    
    # Check whether qemu is already running
    qemu_pids = find_qemu_pids()
    if qemu_pids:
        eprint(f"\nError: \033[46m\033[1;93mRunning instance{"s" if len(qemu_pids) == 1 else ""} of QEMU\033[0m is detected:")
        for pid in qemu_pids:
            eprint(f" - {pid}")
        eprint(f"Please stop above process{"es" if len(qemu_pids) == 1 else ""} before running the grader.")
        sys.exit(1)

    # Prepare run environment
    if os.path.exists(grub_files_dir):
        shutil.rmtree(grub_files_dir)

    os.makedirs(os.path.join(grub_files_dir, 'boot/grub'), exist_ok=True)
    shutil.copy(kernel_path, os.path.join(grub_files_dir, 'boot/keos'))
    shutil.move(kernel_path, 'keos_kernel')

    # Setup Control + C handler
    def handler(signum, frame):
        print('\nrun.py is interrupted. Exiting process...')
        sys.exit(0)
    signal.signal(signal.SIGINT, handler)

    # Handle arguments
    pj = int(current_path.split('/')[-2][-1])
    run_command = make_run_command(pj)
    mode = sys.argv[2] if len(sys.argv) > 2 else 'run'
    filt = sys.argv[2:] if mode != 'grade' else []

    # Setup gdbs
    if 'GDB' in os.environ:
        if len(filt) == 0:
            eprint("Error: You can only use the GDB when running a single test. Please provide the test's name.")
            sys.exit(1)
        else:
            run_command = setup_gdb(run_command)

    # Resolve configurations
    conf = json.loads(open('.grade-target', 'r').read())
    whitelist = conf['whitelist']
    rubrics = conf['rubrics']

    if mode == 'grade':
        check_whitelist_compliance(whitelist)
        grading_result = grade(pj, run_command, rubrics)
        summarize(pj, rubrics, grading_result)
    elif pj == 5:
        grade(pj, run_command, rubrics, filt)
    else:
        run(pj, run_command)

if __name__ == '__main__':
    main()
