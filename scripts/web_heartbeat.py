#!/usr/bin/env python3
import argparse
import sys
import time
import datetime as dt
import webbrowser
from typing import List

try:
    import requests
except Exception:
    print("ERROR: This script requires 'requests' (pip install requests)", file=sys.stderr)
    sys.exit(1)


def now_str() -> str:
    return dt.datetime.now().strftime('%Y-%m-%dT%H:%M:%S')


def http_get(url: str, timeout: float = 2.0) -> int:
    try:
        r = requests.get(url, timeout=timeout, allow_redirects=True)
        return r.status_code
    except Exception:
        return 0


def main():
    parser = argparse.ArgumentParser(description='ESP32 Web Heartbeat & Page Cycle')
    parser.add_argument('--url', default='http://10.27.27.201', help='Base device URL (default: http://10.27.27.201)')
    parser.add_argument('--heartbeat-interval', type=float, default=2.0, help='Seconds between heartbeats (default: 2)')
    parser.add_argument('--cycle-interval', type=float, default=10.0, help='Seconds between page loads (default: 10)')
    parser.add_argument('--open-each', action='store_true', help='Open each page in the default browser as it cycles')
    parser.add_argument('--open-on-failure', action='store_true', help='Open the failing page in browser on failure')
    parser.add_argument('--once', action='store_true', help='Run one cycle then exit')
    args = parser.parse_args()

    base = args.url.rstrip('/')
    ping_url = f"{base}/ping"
    health_url = f"{base}/health"

    pages: List[str] = [
        '/', '/dashboard', '/logs', '/files', '/ota', '/control', '/dev', '/graphs'
    ]

    last_cycle = 0.0
    cycle_index = 0
    failures = 0

    print(f"{now_str()} Starting heartbeat for {base}")
    try:
        while True:
            t = time.time()

            # Heartbeat: prefer /ping, fallback to /health
            status = http_get(ping_url, timeout=1.5)
            if status == 0:
                status = http_get(health_url, timeout=2.0)
            ok = (200 <= status < 300)
            print(f"{now_str()} HB {status}")

            # Periodically load next page
            if t - last_cycle >= args.cycle_interval:
                page = pages[cycle_index % len(pages)]
                url = f"{base}{page}"
                cycle_index += 1
                last_cycle = t
                print(f"{now_str()} GET {url}")

                st = http_get(url, timeout=4.0)
                print(f"{now_str()} ST {st} {url}")
                if args.open_each:
                    try:
                        webbrowser.open_new_tab(url)
                    except Exception:
                        pass

                # Post-GET heartbeat to detect crash induced by page
                status2 = http_get(ping_url, timeout=1.5) or http_get(health_url, timeout=2.0)
                ok2 = (200 <= status2 < 300)
                if not ok2:
                    failures += 1
                    print(f"{now_str()} FAIL after {url} (HB={status2})")
                    if args.open_on_failure:
                        try:
                            webbrowser.open_new_tab(url)
                        except Exception:
                            pass

                if args.once and cycle_index >= len(pages):
                    break

            time.sleep(args.heartbeat_interval)

    except KeyboardInterrupt:
        print(f"{now_str()} Stopped. Total failures: {failures}")


if __name__ == '__main__':
    main()


