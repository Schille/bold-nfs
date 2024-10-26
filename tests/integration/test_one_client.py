import os
import random
import string
from tempfile import TemporaryDirectory
from time import sleep


def test_a_list_directory(mount_client: TemporaryDirectory):
    dirs = os.listdir(mount_client.name)
    assert set(["home", "init", "etc"]) == set(dirs)

def test_b_write_file(mount_client: TemporaryDirectory):
    name = os.path.join(mount_client.name, "file1")
    file = open(name, "w")
    file.writelines(["Hello world"])
    file.flush()
    file.close()

    file = open(name, "r")
    line = file.readline()
    assert line == "Hello world"
    file.close()

    dirs = os.listdir(mount_client.name)
    assert set(["home", "init", "etc", "file1"]) == set(dirs)

def test_c_delete_file(mount_client: TemporaryDirectory):
    name = os.path.join(mount_client.name, "file1")
    file = open(name, "w")
    file.close()
    os.unlink(name)

    dirs = os.listdir(mount_client.name)
    assert set(["home", "init", "etc"]) == set(dirs)

def test_d_mkdir(mount_client: TemporaryDirectory):
    os.mkdir(os.path.join(mount_client.name, "mydir"))
    os.mkdir(os.path.join(mount_client.name, "mydir", "mydir2"))
    os.mkdir(os.path.join(mount_client.name, "mydir", ".mydir2"))
    os.mkdir(os.path.join(mount_client.name, "mydir", "_+-*mydir3"))

    dirs = os.listdir(mount_client.name)
    assert set(["home", "init", "etc", "mydir"]) == set(dirs)

    dirs = os.listdir(os.path.join(mount_client.name, "mydir"))
    assert set(["mydir2", ".mydir2", "_+-*mydir3"]) == set(dirs)

def test_e_mkdir_addfile(mount_client: TemporaryDirectory):
    os.mkdir(os.path.join(mount_client.name, "mydir4"))

    name = os.path.join(mount_client.name, "mydir4", ".file1")
    file = open(name, "w")
    file.writelines(["Hello world"])
    file.flush()
    file.close()

    dirs = os.listdir(os.path.join(mount_client.name, "mydir4"))
    assert set([".file1"]) == set(dirs)

    file = open(name, "r")
    line = file.readline()
    assert line == "Hello world"
    file.close()


def test_f_mkdir_largefile(mount_client: TemporaryDirectory):
    os.mkdir(os.path.join(mount_client.name, "mydir5"))

    name = os.path.join(mount_client.name, "mydir5", "file1")
    file = open(name, "wb")
    content = "".join(random.choice(string.ascii_lowercase) for i in range(10**7))
    file.write(content.encode("utf-8"))
    
    file.flush()
    file.close()
    file_stats = os.stat(name)
    assert int(file_stats.st_size / (1024 * 1024)) == 9

    
