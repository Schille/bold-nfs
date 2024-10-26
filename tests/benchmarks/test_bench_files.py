import re
import subprocess
from tempfile import TemporaryDirectory
from time import sleep, time
import pytest


def dd_write_file(temp_dir: str, size_in_mb: str, oflag: str):
    cmd = ["dd", "if=/dev/zero", f"of={temp_dir}/{size_in_mb}mb.img", f"bs={size_in_mb}MB", "count=1", f"oflag={oflag}"]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

def dd_read_file(temp_dir: str, filename: str):
    cmd = ["dd", f"if={temp_dir}/{filename}", "of=/dev/null", f"bs=1M"]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


@pytest.mark.benchmark(
    group="Write 10 MB file (dd)",
    max_time=190,
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_10mb_dsync(mount_client: TemporaryDirectory, benchmark):
    benchmark.pedantic(dd_write_file, args=(mount_client.name, "10", "dsync"), iterations=1, rounds=20)

@pytest.mark.benchmark(
    group="Write 10 MB file (dd)",
    max_time=190,
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_10mb_direct(mount_client: TemporaryDirectory, benchmark):
    benchmark.pedantic(dd_write_file, args=(mount_client.name, "10", "direct"), iterations=1, rounds=20)


@pytest.mark.benchmark(
    group="Write 10 MB file (dd)",
    max_time=190,
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_10mb_sync(mount_client: TemporaryDirectory, benchmark):
    benchmark.pedantic(dd_write_file, args=(mount_client.name, "10", "sync"), iterations=1, rounds=20)

@pytest.mark.benchmark(
    group="Read 100 MB file (dd)",
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_read_100mb(mount_client: TemporaryDirectory, benchmark):
    dd_write_file(mount_client.name, "100", "sync")
    benchmark.pedantic(dd_read_file, args=(mount_client.name, "100mb.img"), iterations=1, rounds=20)

@pytest.mark.benchmark(
    group="Write 100 MB file (dd)",
    max_time=2000,
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_100mb_dsync(mount_client: TemporaryDirectory, benchmark):
    benchmark.pedantic(dd_write_file, args=(mount_client.name, "100", "dsync"), iterations=1, rounds=20)

@pytest.mark.benchmark(
    group="Write 100 MB file (dd)",
    max_time=2000,
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_100mb_direct(mount_client: TemporaryDirectory, benchmark):
    benchmark.pedantic(dd_write_file, args=(mount_client.name, "100", "direct"), iterations=1, rounds=20)

@pytest.mark.benchmark(
    group="Write 100 MB file (dd)",
    max_time=2000,
    timer=time,
    disable_gc=True,
    warmup=False
)
def test_100mb_sync(mount_client: TemporaryDirectory, benchmark):
    benchmark.pedantic(dd_write_file, args=(mount_client.name, "100", "sync"), iterations=1, rounds=20)