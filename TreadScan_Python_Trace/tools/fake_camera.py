"""
Simulated Zebra Industrial Barcode Scanner (FS42 / Aurora)
Continuous Non-Blocking Streamer:
- Binds to 0.0.0.0:22172
- Continuously scans and generates barcodes every 3 seconds regardless of whether a client is connected.
- Streams live barcode scans to any connected client (LabVIEW / Python listener).
"""

import socket
import select
import time
import sys

HOST = '0.0.0.0'
PORT = 22172

def get_local_ips():
    ips = []
    try:
        # Fetch hostname IPs
        hostname = socket.gethostname()
        for ip in socket.gethostbyname_ex(hostname)[2]:
            if not ip.startswith("127."):
                ips.append(ip)
    except:
        pass
    return ips if ips else ["127.0.0.1"]

def run_fake_camera():
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server_socket.setblocking(False)

    local_ips = get_local_ips()
    ip_list_str = ", ".join(local_ips)

    try:
        server_socket.bind((HOST, PORT))
        server_socket.listen(5)
        print(f"\033[94m=======================================================\033[0m")
        print(f"\033[94m=== ZEBRA FS42 CAMERA ONLINE & STREAMING ON PORT {PORT} ===\033[0m")
        print(f"\033[94m=== Camera IP Address(es): {ip_list_str:<26} ===\033[0m")
        print(f"\033[94m=======================================================\033[0m\n")
    except Exception as e:
        print(f"\033[91m[ERROR] Failed to bind server socket: {e}\033[0m")
        sys.exit(1)

    clients = []
    read_count = 1

    while True:
        try:
            # Check for new client connections without blocking
            readable, _, _ = select.select([server_socket], [], [], 0.05)
            if readable:
                client_conn, client_addr = server_socket.accept()
                client_conn.setblocking(False)
                clients.append(client_conn)
                print(f"\033[92m[CAMERA EVENT] Client connected from {client_addr}. Connected clients: {len(clients)}\033[0m")

            # Generate barcode scan continuously
            code_str = f"SE07004H3Y_READ_{read_count}\r\n"
            code_bytes = code_str.encode('ascii')
            print(f"\033[96m[SCANNER ACTIVE] Barcode Scanned #{read_count}: {code_str.strip()} (Active Clients: {len(clients)})\033[0m")
            read_count += 1

            # Stream barcode to all active client connections
            disconnected_clients = []
            for c in clients:
                try:
                    c.sendall(code_bytes)
                except Exception:
                    disconnected_clients.append(c)

            # Clean up dropped client connections
            for c in disconnected_clients:
                try:
                    c.close()
                except:
                    pass
                if c in clients:
                    clients.remove(c)
                print(f"\033[91m[CAMERA EVENT] Client connection dropped. Remaining clients: {len(clients)}\033[0m")

            time.sleep(3)

        except Exception as e:
            print(f"[CAMERA ERROR] {e}")
            time.sleep(1)

if __name__ == "__main__":
    try:
        run_fake_camera()
    except KeyboardInterrupt:
        print("\n[MOCK CAMERA] Server stopped by user.")


