from time import sleep
import pytest
import subprocess, os, tempfile

@pytest.fixture(scope="session",)
def bold_mem_release_build():
    r = subprocess.run(["cargo", "build", "-p", "bold-mem", "--release"])
    assert r.returncode == 0

@pytest.fixture(scope="module")
def mount_client(bold_mem_release_build):
    # bold server
    bold_mem = [str(os.path.join("target", "release", "bold-mem"))]
    args = [str(os.path.join("tests", "memoryfs.yaml"))]
    proc = subprocess.Popen(bold_mem + args)
    sleep(0.5)
    assert proc.poll() is None # running...
    # nfs client
    tmp = tempfile.TemporaryDirectory()
    r = subprocess.run(["sudo", "mount.nfs4", "-n", "-o" "fg,soft,sec=none,vers=4.0,port=11112", "127.0.0.1:/", tmp.name])
    assert r.returncode == 0
    yield tmp
    try:
        subprocess.run(["sudo", "umount", "-f", tmp.name], timeout=2)
    except:
        sleep(5)
    proc.kill()
    tmp.cleanup()
    
            
    
        
    