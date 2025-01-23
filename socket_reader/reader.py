import json
import socket
from collections import defaultdict
import sys

serial_stats = defaultdict(lambda: {"buf_count": 0, "rct_fail_count": 0, "apt_fail_count": 0})

def update_serial_stats(data):
    try:
        parsed_data = json.loads(data)
        serial = parsed_data.get("serial")
        raw_data = parsed_data.get("data", {}).get("RawStream", {})
        
        if serial:
            serial_stats[serial]["buf_count"] += len(raw_data.get("buf", []))
            if raw_data.get("rct_fail"):
                serial_stats[serial]["rct_fail_count"] += 1
            if raw_data.get("apt_fail"):
                serial_stats[serial]["apt_fail_count"] += 1
    except json.JSONDecodeError as e:
        print(f"JSON decoding error: {e}")

def display_stats():
    sys.stdout.write("\r")
    stats = " | ".join(
        f"{serial}: seeds={info['buf_count']}, rct_fail={info['rct_fail_count']}, apt_fail={info['apt_fail_count']}" 
        for serial, info in serial_stats.items()
    )
    sys.stdout.write(stats)
    sys.stdout.flush()

def read_from_socket(sock):
    buffer = ""
    while True:
        data = sock.recv(1024).decode("utf-8")
        if not data:
            break
        
        buffer += data
        while "\n" in buffer:
            line, buffer = buffer.split("\n", 1)
            update_serial_stats(line)
            display_stats()

if __name__ == "__main__":
    HOST = "127.69.42.0"
    PORT = 1412
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((HOST, PORT))
        try:
            read_from_socket(s)
        except Exception as e:
            print(f"\nAn error occurred: {e}")
