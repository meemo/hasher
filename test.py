import os
import shutil
import subprocess
import tempfile
import gzip
import http.server
import socketserver
import json


CURRENT_DIR = os.getcwd()
TEST_FILES_DIR = os.path.join(CURRENT_DIR, "test")

BUILD_COMMAND = ["cargo", "build"]
TEST_COMMAND = ["cargo", "test", "--", "--nocapture"]


def create_test_files():
    """Set up test files and get known hashes"""
    os.makedirs(TEST_FILES_DIR, exist_ok=True)

    # Create some test files
    files = {
        "empty.txt": b"",
        "small.txt": b"test data",
        "medium.txt": b"test data" * 1000,
        "large.txt": b"test data" * 100000
    }

    for name, content in files.items():
        path = os.path.join(TEST_FILES_DIR, name)
        with open(path, "wb") as f:
            f.write(content)

        # Create compressed version
        gz_path = path + ".gz"
        with gzip.open(gz_path, "wb") as f:
            f.write(content)

    print(f"Created test files in {TEST_FILES_DIR}")


def run_hasher(*args, check_output=True):
    """Run hasher with given args and return output"""
    cmd = ["./target/debug/hasher"] + list(args)
    print(f"\nExecuting: {' '.join(cmd)}")

    result = subprocess.run(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    if result.returncode != 0 and check_output:
        print(f"\nCommand failed with return code {result.returncode}")
        print("\nSTDOUT:")
        print(result.stdout)
        print("\nSTDERR:")
        print(result.stderr)

    return result


def test_hash():
    """Test basic hashing functionality"""
    print("\nTesting hash command...")

    result = run_hasher("hash", TEST_FILES_DIR)
    assert result.returncode == 0, "Basic hash failed"

    result = run_hasher("hash", "--json-only", TEST_FILES_DIR)
    assert result.returncode == 0, "JSON-only hash failed"
    assert result.stdout.strip(), "JSON output is empty"

    # Validate JSON output
    try:
        for line in result.stdout.splitlines():
            if line.strip():
                json.loads(line)
    except json.JSONDecodeError:
        raise AssertionError("Invalid JSON output")

    result = run_hasher("hash", "--sql-only", TEST_FILES_DIR)
    assert result.returncode == 0, "SQL-only hash failed"


def test_verify():
    """Test verification of hashed files"""
    print("\nTesting verify command...")

    # First hash the files
    result = run_hasher("hash", TEST_FILES_DIR)
    assert result.returncode == 0, "Initial hash before verify failed"

    # Verify all files
    result = run_hasher("verify")
    assert result.returncode == 0, "Verify all files failed"

    # Modify a file and verify again
    test_file = os.path.join(TEST_FILES_DIR, "small.txt")
    print(f"\nModifying test file: {test_file}")
    with open(test_file, "wb") as f:
        f.write(b"modified data")

    result = run_hasher("verify", "--mismatches-only")
    assert result.returncode == 0, "Verify with mismatches failed"
    assert "small.txt" in result.stdout, "Modified file not detected"


def test_copy():
    """Test copying functionality"""
    print("\nTesting copy command...")

    with tempfile.TemporaryDirectory() as tmpdir:
        print(f"\nCopying to temp dir: {tmpdir}")

        # Test without compression
        result = run_hasher("copy", TEST_FILES_DIR, tmpdir)
        assert result.returncode == 0, f"Copy failed\nStdout: {result.stdout}\nStderr: {result.stderr}"

        # Verify files were copied correctly
        missing_files = []
        for f in os.listdir(TEST_FILES_DIR):
            if not f.endswith(".gz"):
                dest_file = os.path.join(tmpdir, f)
                if not os.path.exists(dest_file):
                    missing_files.append(f)

        assert not missing_files, f"Files not copied: {missing_files}"

        # Test with compression
        compressed_dir = os.path.join(tmpdir, "compressed")
        os.makedirs(compressed_dir, exist_ok=True)
        result = run_hasher("copy", "--compress", TEST_FILES_DIR, compressed_dir)
        assert result.returncode == 0, f"Compressed copy failed\nStdout: {result.stdout}\nStderr: {result.stderr}"

        # Verify compressed files exist
        missing_compressed = []
        for f in os.listdir(TEST_FILES_DIR):
            if not f.endswith(".gz"):
                compressed_file = os.path.join(compressed_dir, f + ".gz")
                if not os.path.exists(compressed_file):
                    missing_compressed.append(f + ".gz")

        assert not missing_compressed, f"Compressed files not created: {missing_compressed}"


def test_download():
    """Test downloading functionality"""
    print("\nTesting download command...")

    with tempfile.TemporaryDirectory() as tmpdir:
        print(f"\nDownloading to temp dir: {tmpdir}")

        # Create necessary subdirectories
        raw_github_dir = os.path.join(tmpdir, "raw.githubusercontent.com")
        os.makedirs(raw_github_dir, exist_ok=True)

        # Test single file download from GitHub raw
        url = "https://raw.githubusercontent.com/rust-lang/rust/master/README.md"
        result = run_hasher("download", url, tmpdir, check_output=True)
        assert result.returncode == 0, f"Single file download failed\nStdout: {result.stdout}\nStderr: {result.stderr}"

        # The file should be in raw.githubusercontent.com/rust-lang/rust/master/README.md
        expected_path = os.path.join(raw_github_dir, "rust-lang", "rust", "master", "README.md")
        assert os.path.exists(expected_path), f"Downloaded file not found at: {expected_path}"
        assert os.path.getsize(expected_path) > 0, "Downloaded file is empty"

        # Test multiple file download from list
        url_file = os.path.join(tmpdir, "urls.txt")
        with open(url_file, "w") as f:
            f.write("https://raw.githubusercontent.com/rust-lang/rust/master/CONTRIBUTING.md\n")
            f.write("https://raw.githubusercontent.com/rust-lang/rust/master/COPYRIGHT\n")

        result = run_hasher("download", url_file, tmpdir)
        assert result.returncode == 0, "Multiple file download failed"

        # Verify downloaded files exist in their proper paths
        for fname in ["CONTRIBUTING.md", "COPYRIGHT"]:
            expected = os.path.join(raw_github_dir, "rust-lang", "rust", "master", fname)
            assert os.path.exists(expected), f"Downloaded file not found at: {expected}"
            assert os.path.getsize(expected) > 0, f"Downloaded file is empty: {expected}"

        # Test downloading with compression
        compressed_dir = os.path.join(tmpdir, "compressed")
        os.makedirs(compressed_dir, exist_ok=True)

        # Create compressed github subdirectory
        compressed_gh_dir = os.path.join(compressed_dir, "raw.githubusercontent.com", "rust-lang", "rust", "master")
        os.makedirs(compressed_gh_dir, exist_ok=True)

        result = run_hasher("download", "--compress", url, compressed_dir)
        assert result.returncode == 0, "Compressed download failed"

        compressed_path = os.path.join(compressed_gh_dir, "README.md.gz")
        assert os.path.exists(compressed_path), f"Compressed file not found at: {compressed_path}"
        assert os.path.getsize(compressed_path) > 0, "Compressed file is empty"

        # Test --no-clobber
        result = run_hasher("download", "--no-clobber", url, tmpdir)
        assert result.returncode == 0, "No-clobber download failed"


def main():
    print("\nRunning hasher tests...")

    if subprocess.run(BUILD_COMMAND).returncode != 0:
        print("Build failed")
        exit(1)

    try:
        # Set up test environment
        create_test_files()

        # Run individual test functions
        test_hash()
        test_verify()
        test_copy()
        test_download()

    except AssertionError as e:
        print(f"\nTest failed: {str(e)}")
        exit(1)
    except Exception as e:
        print(f"\nUnexpected error: {str(e)}")
        exit(1)
    finally:
        print(f"\nCleaning up {TEST_FILES_DIR}")
        shutil.rmtree(TEST_FILES_DIR, ignore_errors=True)

    print("\nAll tests passed!")
    exit(0)


if __name__ == "__main__":
    main()
