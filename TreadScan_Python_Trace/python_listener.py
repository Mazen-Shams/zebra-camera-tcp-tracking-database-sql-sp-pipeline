"""
Production-Grade Multi-Camera Async Barcode Listener Service
- Manages 27 Zebra Industrial Barcode Scanners (IP Range 192.168.1.80-110, Port 22172)
- Solves LabVIEW Half-Open Socket Deadlock via OS-Level TCP Keep-Alive
- Asynchronous Non-Blocking Event Loop (asyncio)
- Integrated Sub-Millisecond Web UI Dashboard on Port 8080
"""

import asyncio
import socket
import struct
import configparser
import os
import sys
import time
import uuid
import json
from datetime import datetime
from collections import deque

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

from aiohttp import web, WSMsgType
from tools.logger import log_camera_error, log_audit_trace, log_system_event, ensure_log_headers

DEFAULT_PORT = 22172
WEB_PORT = 8080
BASE_DIR = os.path.dirname(__file__)
INI_PATH = os.path.join(BASE_DIR, 'devices.ini')
DASHBOARD_HTML_PATH = os.path.join(BASE_DIR, 'dashboard.html')

# Terminal Output Mode: Default is QUIET. Pass --verbose or -v to print every barcode read line.
IS_VERBOSE = '--verbose' in sys.argv or '-v' in sys.argv

# Global State Container
device_states = {}
devices_order = []
recent_events = deque(maxlen=20)
stats = {'total_reads': 0, 'good_reads': 0, 'bad_reads': 0, 'db_success': 0}
ws_clients = set()

def load_devices_config():
    """
    Parses devices.ini and returns a list of dictionaries:
    [{'machine_code': 'TRGB1', 'ip': '192.168.1.65'}, ...]
    """
    global devices_order
    devices = []
    devices_order.clear()
    if os.path.exists(INI_PATH):
        config = configparser.ConfigParser()
        config.read(INI_PATH)
        for section in config.sections():
            if section.lower().startswith('device'):
                m_code = config.get(section, 'Machine_Code', fallback='').strip('"\' ')
                ip_addr = config.get(section, 'IP_Address', fallback='').strip('"\' ')
                if m_code and ip_addr:
                    devices.append({'machine_code': m_code, 'ip': ip_addr})
                    devices_order.append(m_code)
    return devices

async def broadcast_ws(message: dict):
    """Broadcasts a JSON message to all connected WebSocket dashboard clients."""
    if not ws_clients:
        return
    msg_str = json.dumps(message)
    dead_clients = set()
    for ws in ws_clients:
        try:
            await ws.send_str(msg_str)
        except Exception:
            dead_clients.add(ws)
    ws_clients.difference_update(dead_clients)

def get_state_snapshot():
    """Generates snapshot of all 27 camera states and total scan statistics."""
    connected_count = sum(1 for d in device_states.values() if d['status'] == 'CONNECTED')
    return {
        'type': 'SNAPSHOT',
        'devices': device_states,
        'devices_order': devices_order,
        'stats': {
            'total_devices': len(device_states),
            'connected': connected_count,
            'total_reads': stats['total_reads'],
            'good_reads': stats['good_reads'],
            'bad_reads': stats['bad_reads'],
            'db_success': stats['db_success']
        }
    }

from db_manager import db_manager

async def async_db_worker(scan_event_id: str, machine_code: str, barcode_str: str, timestamp: str, host: str):
    """Background worker for non-blocking DB execution. Broadcasts detailed results back to UI."""
    success, db_msg = await asyncio.to_thread(db_manager.execute_stored_procedure, machine_code, barcode_str)
    if success:
        stats['db_success'] += 1
    
    # Broadcast detailed DB audit result back to Web UI log feed
    db_result_event = {
        'type': 'DB_RESULT_EVENT',
        'event_id': scan_event_id,
        'timestamp': timestamp,
        'machine_code': machine_code,
        'host': host,
        'barcode': barcode_str,
        'success': success,
        'db_status': db_msg
    }
    await broadcast_ws(db_result_event)
    await broadcast_ws(get_state_snapshot())

async def process_barcode_queue(queue: asyncio.Queue):
    """
    Central consumer queue worker for handling incoming barcodes asynchronously from all machines.
    Broadcasts scan events INSTANTLY to Web UI without waiting for DB network calls.
    """
    while True:
        timestamp, machine_code, host, barcode_str = await queue.get()
        stats['total_reads'] += 1
        
        # Update device last barcode state
        if machine_code in device_states:
            device_states[machine_code]['last_barcode'] = barcode_str
            device_states[machine_code]['last_time'] = timestamp

        event_id = f"evt_{uuid.uuid4().hex[:8]}"

        if barcode_str.upper() in ["NOREAD", "BAD_READ", "FAIL"]:
            stats['bad_reads'] += 1
            db_msg = "BAD READ (DB SKIPPED)"
            log_audit_trace(machine_code, barcode_str, db_msg)
        else:
            stats['good_reads'] += 1
            db_msg = "DB EXECUTION QUEUED"
            # Launch DB execution in background task so UI updates INSTANTLY
            asyncio.create_task(async_db_worker(event_id, machine_code, barcode_str, timestamp, host))

        # Broadcast scan event INSTANTLY to Web UI with unique event_id
        scan_event = {
            'type': 'SCAN_EVENT',
            'event_id': event_id,
            'timestamp': timestamp,
            'machine_code': machine_code,
            'host': host,
            'barcode': barcode_str,
            'db_status': db_msg
        }
        await broadcast_ws(scan_event)
        await broadcast_ws(get_state_snapshot())
            
        queue.task_done()

async def listen_to_device(machine_code: str, host: str, port: int, queue: asyncio.Queue):
    """
    Independent worker task for a single camera / machine.
    Handles socket creation, TCP Keep-Alive, line reading, and auto-reconnection.
    """
    reconnect_attempts = 0

    if machine_code not in device_states:
        device_states[machine_code] = {
            'ip': host,
            'status': 'INIT',
            'keepalive': False,
            'retry': 0,
            'last_barcode': 'N/A',
            'last_time': 'N/A'
        }

    while True:
        writer = None
        try:
            device_states[machine_code]['status'] = 'CONNECTING'
            device_states[machine_code]['retry'] = reconnect_attempts
            await broadcast_ws(get_state_snapshot())
            
            # Connect to camera socket asynchronously with 5.0s timeout to allow TCP SYN retransmissions
            reader, writer = await asyncio.wait_for(asyncio.open_connection(host, port), timeout=5.0)

            # Enable Socket-Level TCP Keep-Alive
            try:
                sock = writer.get_extra_info('socket')
                if sock is not None:
                    raw_sock = getattr(sock, '_sock', sock)
                    raw_sock.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)
                    if hasattr(socket, 'SIO_KEEPALIVE_VALS'):
                        raw_sock.ioctl(socket.SIO_KEEPALIVE_VALS, (1, 5000, 1000))
                    device_states[machine_code]['keepalive'] = True
            except Exception:
                device_states[machine_code]['keepalive'] = False

            if reconnect_attempts > 0:
                log_camera_error(machine_code, host, f"[RECOVERED] Connected successfully after {reconnect_attempts} retry attempts.")
                reconnect_attempts = 0

            device_states[machine_code]['status'] = 'CONNECTED'
            device_states[machine_code]['retry'] = 0
            await broadcast_ws(get_state_snapshot())
            
            print(f"\033[92m[CONNECTED] Machine: {machine_code} ({host}:{port}) | Keep-Alive: ACTIVE\033[0m")

            while True:
                line = await reader.readline()
                if not line:
                    raise ConnectionResetError("Remote scanner closed TCP connection")

                barcode_str = line.decode('utf-8', errors='ignore').strip()
                if barcode_str:
                    timestamp = datetime.now().strftime('%H:%M:%S - %d/%m')
                    if IS_VERBOSE:
                        print(f"\033[96m[{timestamp}] {machine_code} ({host}) -> READ: {barcode_str}\033[0m")
                    await queue.put((timestamp, machine_code, host, barcode_str))

        except Exception as e:
            reconnect_attempts += 1
            device_states[machine_code]['status'] = f'RECONNECTING ({reconnect_attempts})'
            device_states[machine_code]['retry'] = reconnect_attempts
            await broadcast_ws(get_state_snapshot())
            
            # Log initial disconnect (Attempt #1) or periodic summary (every 100 attempts) to avoid log flood
            if reconnect_attempts == 1 or reconnect_attempts % 100 == 0:
                err_name = type(e).__name__
                err_str = str(e).strip()
                err_desc = f"[{err_name}] {err_str}" if err_str else f"[{err_name}] Socket connection timed out"
                log_camera_error(machine_code, host, f"Connection error (Attempt #{reconnect_attempts}): {err_desc}")

        finally:
            if writer:
                try:
                    # Set SO_LINGER timeout=0 to force TCP RST and release camera port immediately
                    sock = writer.get_extra_info('socket')
                    if sock is not None:
                        raw_sock = getattr(sock, '_sock', sock)
                        raw_sock.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack('ii', 1, 0))
                except Exception:
                    pass
                writer.close()
                try:
                    await writer.wait_closed()
                except Exception:
                    pass

        # Exponential backoff capped at 5.0s (capped exponent prevents integer OverflowError on long disconnects)
        safe_exp = min(max(reconnect_attempts - 1, 0), 6)
        backoff = min(0.5 * (2 ** safe_exp), 5.0)
        try:
            await asyncio.sleep(backoff)
        except asyncio.CancelledError:
            break
        except Exception:
            await asyncio.sleep(1.0)

async def handle_index(request):
    """Serves real-time dark-mode web dashboard HTML."""
    if os.path.exists(DASHBOARD_HTML_PATH):
        return web.FileResponse(DASHBOARD_HTML_PATH)
    return web.Response(text="Dashboard HTML file not found", status=404)

async def handle_websocket(request):
    ws = web.WebSocketResponse()
    await ws.prepare(request)
    ws_clients.add(ws)

    # Send initial state snapshot on connect
    await ws.send_str(json.dumps(get_state_snapshot()))

    try:
        async for msg in ws:
            if msg.type == WSMsgType.TEXT:
                pass
    finally:
        ws_clients.remove(ws)
    return ws

async def start_web_server():
    env_port = int(os.getenv('WEB_PORT', '8088'))
    app = web.Application()
    app.router.add_get('/', handle_index)
    app.router.add_get('/ws', handle_websocket)

    runner = web.AppRunner(app)
    await runner.setup()

    active_port = env_port
    for offset in range(20):
        try:
            target_port = env_port + offset
            site = web.TCPSite(runner, '0.0.0.0', target_port)
            await site.start()
            active_port = target_port
            break
        except OSError as e:
            if offset == 19:
                raise e
            continue

    print(f"\033[92m[WEB SERVER] Live UI Dashboard running at http://localhost:{active_port}\033[0m")
    log_system_event(f"Web UI Dashboard running at http://localhost:{active_port}")

active_camera_tasks = {}

async def watch_devices_config(queue: asyncio.Queue):
    """
    Monitors devices.ini for changes on the fly and acts as a worker watchdog.
    Dynamically updates camera tasks & IP addresses without restarting the service,
    and automatically resurrects any worker task that crashed or exited unexpectedly.
    """
    last_mtime = 0
    while True:
        try:
            current_devices = load_devices_config()
            current_codes = {d['machine_code']: d['ip'] for d in current_devices}

            # --- 1. WATCHDOG: Detect and resurrect any dead/crashed camera worker tasks ---
            for d in current_devices:
                code = d['machine_code']
                ip = d['ip']
                if code in active_camera_tasks and active_camera_tasks[code].done():
                    exc = None
                    try:
                        if not active_camera_tasks[code].cancelled():
                            exc = active_camera_tasks[code].exception()
                    except Exception:
                        pass
                    log_system_event(f"[WATCHDOG] Resurrecting worker task for {code} ({ip}) - Task ended with: {exc}")
                    print(f"\033[93m[WATCHDOG] Resurrecting worker task for {code} ({ip}) - Reason: {exc}\033[0m")
                    task = asyncio.create_task(listen_to_device(code, ip, DEFAULT_PORT, queue))
                    active_camera_tasks[code] = task

            # --- 2. CONFIG HOT-RELOAD: Check for devices.ini file modifications ---
            if os.path.exists(INI_PATH):
                mtime = os.path.getmtime(INI_PATH)
                if mtime != last_mtime:
                    last_mtime = mtime

                    # 1. Update or spawn new listeners
                    for d in current_devices:
                        code = d['machine_code']
                        ip = d['ip']
                        if code not in active_camera_tasks or device_states.get(code, {}).get('ip') != ip:
                            if code in active_camera_tasks:
                                active_camera_tasks[code].cancel()
                            
                            device_states[code] = {
                                'ip': ip,
                                'status': 'INIT',
                                'keepalive': False,
                                'retry': 0,
                                'last_barcode': 'N/A',
                                'last_time': 'N/A'
                            }
                            task = asyncio.create_task(listen_to_device(code, ip, DEFAULT_PORT, queue))
                            active_camera_tasks[code] = task

                    # 2. Cancel removed targets
                    removed_codes = set(active_camera_tasks.keys()) - set(current_codes.keys())
                    for code in removed_codes:
                        active_camera_tasks[code].cancel()
                        del active_camera_tasks[code]
                        if code in device_states:
                            del device_states[code]

                    await broadcast_ws(get_state_snapshot())
        except Exception:
            pass
        await asyncio.sleep(3)

async def main():
    print("=========================================================================")
    print(" Production-Grade Multi-Camera Async Barcode Listener Service")
    print("=========================================================================")
    
    try:
        from tools.logger import ensure_log_headers
        ensure_log_headers()
    except Exception:
        pass
    all_devices = load_devices_config()
    print(f"[CONFIG] Loaded {len(all_devices)} camera machine targets from devices.ini")
    log_system_event(f"Multi-Camera Listener Service initialized with {len(all_devices)} camera targets.")
    
    await start_web_server()

    queue = asyncio.Queue()

    # Spawn worker queue consumer
    asyncio.create_task(process_barcode_queue(queue))

    # Spawn dynamic devices.ini config watcher (Hot-Reload on the fly)
    asyncio.create_task(watch_devices_config(queue))

    # Keep service running indefinitely
    await asyncio.Event().wait()

if __name__ == '__main__':
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\033[93m[SHUTDOWN] Listener service stopped by user.\033[0m")

