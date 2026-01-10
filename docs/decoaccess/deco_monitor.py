#!/usr/bin/env python3
"""
TP-Link DECO Router Monitor Script
Uses tplinkrouterc6u library for authenticated API access

Usage:
    python3 deco_monitor.py [--json] [--config CONFIG_FILE]
"""

import argparse
import json
import sys
import os
from datetime import datetime
from base64 import b64decode
from tplinkrouterc6u import TPLinkDecoClient


def load_config(config_path: str) -> dict:
    """Load configuration from JSON file"""
    with open(config_path, 'r') as f:
        return json.load(f)


def get_deco_full_status(host: str, password: str) -> dict:
    """Get all available DECO router information"""
    client = TPLinkDecoClient(host, password)
    client.authorize()

    # 1. Basic status
    status = client.get_status()

    # 2. IPv4 detailed status
    ipv4_status = client.get_ipv4_status()

    # 3. Firmware info
    firmware_info = None
    try:
        firmware = client.get_firmware()
        firmware_info = {
            'model': firmware.model,
            'hardware_version': firmware.hardware_version,
            'software_version': firmware.firmware_version
        }
    except Exception:
        pass

    # 4. Mesh nodes info
    mesh_nodes = []
    for node in client.devices:
        node_info = {
            'nickname': node.get('nickname', ''),
            'mac': node.get('mac', ''),
            'ip': node.get('device_ip', ''),
            'model': node.get('device_model', ''),
            'role': node.get('role', ''),
            'status': node.get('inet_status', ''),
            'group_status': node.get('group_status', ''),
            'hardware_version': node.get('hardware_ver', ''),
            'software_version': node.get('software_ver', ''),
            'device_type': node.get('device_type', ''),
            'signal_level': node.get('signal_level', {}),
            'signal_strength': node.get('signal_strength', {}),
            'connection_type': node.get('connection_type', []),
            'bssid_2g': node.get('bssid_2g', ''),
            'bssid_5g': node.get('bssid_5g', ''),
            'oem_id': node.get('oem_id', ''),
            'hw_id': node.get('hw_id', ''),
        }
        # Decode custom nickname if exists
        if node.get('custom_nickname'):
            try:
                node_info['custom_nickname'] = b64decode(node['custom_nickname']).decode()
            except:
                node_info['custom_nickname'] = node.get('custom_nickname')
        mesh_nodes.append(node_info)

    # 5. Connected devices
    devices = []
    for device in status.devices:
        conn_type = "2.4GHz" if "2G" in str(device.type) else \
                   "5GHz" if "5G" in str(device.type) else \
                   "6GHz" if "6G" in str(device.type) else \
                   "wired" if "WIRED" in str(device.type) else "unknown"

        devices.append({
            'hostname': device.hostname,
            'ip': str(device._ipaddr),
            'mac': str(device._macaddr),
            'connection': conn_type,
            'connection_raw': str(device.type),
            'down_speed': device.down_speed,
            'up_speed': device.up_speed,
            'active': device.active if hasattr(device, 'active') else True
        })

    result = {
        'timestamp': datetime.now().isoformat(),
        'host': host,

        # Network
        'network': {
            'wan': {
                'mac': str(status._wan_macaddr) if status._wan_macaddr else None,
                'ip': str(status._wan_ipv4_addr) if status._wan_ipv4_addr else None,
                'gateway': str(status._wan_ipv4_gateway) if status._wan_ipv4_gateway else None,
                'netmask': str(ipv4_status._wan_ipv4_netmask) if ipv4_status._wan_ipv4_netmask else None,
                'connection_type': ipv4_status._wan_ipv4_conntype if ipv4_status._wan_ipv4_conntype else None,
                'dns1': str(ipv4_status._wan_ipv4_pridns) if ipv4_status._wan_ipv4_pridns else None,
                'dns2': str(ipv4_status._wan_ipv4_snddns) if ipv4_status._wan_ipv4_snddns else None,
            },
            'lan': {
                'mac': str(status._lan_macaddr) if status._lan_macaddr else None,
                'ip': str(status._lan_ipv4_addr) if status._lan_ipv4_addr else None,
                'netmask': str(ipv4_status._lan_ipv4_netmask) if ipv4_status._lan_ipv4_netmask else None,
            }
        },

        # System
        'system': {
            'cpu_usage_percent': round(status.cpu_usage * 100, 1) if status.cpu_usage else None,
            'mem_usage_percent': round(status.mem_usage * 100, 1) if status.mem_usage else None,
        },

        # WiFi
        'wifi': {
            'band_2g': {
                'enabled': status.wifi_2g_enable,
                'guest_enabled': status.guest_2g_enable,
                'iot_enabled': status.iot_2g_enable,
            },
            'band_5g': {
                'enabled': status.wifi_5g_enable,
                'guest_enabled': status.guest_5g_enable,
                'iot_enabled': status.iot_5g_enable,
            },
            'band_6g': {
                'enabled': status.wifi_6g_enable,
                'guest_enabled': status.guest_6g_enable,
                'iot_enabled': status.iot_6g_enable,
            }
        },

        # Client counts
        'clients': {
            'wifi_total': status.wifi_clients_total,
            'wired_total': status.wired_total,
            'guest_total': status.guest_clients_total,
            'iot_total': status.iot_clients_total,
            'total': status.clients_total,
        },

        # Connected devices list
        'devices': devices,

        # Mesh nodes
        'mesh_nodes': mesh_nodes,

        # Firmware
        'firmware': firmware_info,
    }

    client.logout()
    return result


def print_human_readable(data: dict):
    """Print status in human-readable format"""
    print("=" * 70)
    print(f"TP-Link DECO Router Status")
    print(f"Host: {data['host']} | Time: {data['timestamp']}")
    print("=" * 70)

    # Network - WAN
    print(f"\n[WAN]")
    wan = data['network']['wan']
    print(f"  MAC:        {wan['mac']}")
    print(f"  IP:         {wan['ip']}")
    print(f"  Gateway:    {wan['gateway']}")
    print(f"  Netmask:    {wan['netmask']}")
    print(f"  Type:       {wan['connection_type']}")
    print(f"  DNS:        {wan['dns1']} / {wan['dns2']}")

    # Network - LAN
    print(f"\n[LAN]")
    lan = data['network']['lan']
    print(f"  MAC:        {lan['mac']}")
    print(f"  IP:         {lan['ip']}")
    print(f"  Netmask:    {lan['netmask']}")

    # System
    print(f"\n[System]")
    sys_info = data['system']
    print(f"  CPU:        {sys_info['cpu_usage_percent']}%")
    print(f"  Memory:     {sys_info['mem_usage_percent']}%")

    # WiFi
    print(f"\n[WiFi]")
    wifi = data['wifi']
    for band, name in [('band_2g', '2.4GHz'), ('band_5g', '5GHz'), ('band_6g', '6GHz')]:
        b = wifi[band]
        if b['enabled'] is not None:
            status = 'ON' if b['enabled'] else 'OFF'
            guest = 'ON' if b['guest_enabled'] else 'OFF'
            iot = f", IoT:{('ON' if b['iot_enabled'] else 'OFF')}" if b['iot_enabled'] is not None else ''
            print(f"  {name}:      Host:{status}, Guest:{guest}{iot}")

    # Clients
    print(f"\n[Clients]")
    clients = data['clients']
    print(f"  WiFi:       {clients['wifi_total']}")
    print(f"  Wired:      {clients['wired_total']}")
    print(f"  Guest:      {clients['guest_total']}")
    if clients['iot_total']:
        print(f"  IoT:        {clients['iot_total']}")
    print(f"  Total:      {clients['total']}")

    # Firmware
    if data['firmware']:
        print(f"\n[Firmware]")
        fw = data['firmware']
        print(f"  Model:      {fw['model']}")
        print(f"  Hardware:   {fw['hardware_version']}")
        print(f"  Software:   {fw['software_version']}")

    # Mesh Nodes
    print(f"\n[Mesh Nodes] ({len(data['mesh_nodes'])} nodes)")
    print("-" * 70)
    for node in data['mesh_nodes']:
        role_icon = "★" if node['role'] == 'master' else "○"
        status_icon = "●" if node['status'] == 'online' else "○"
        print(f"  {role_icon} {node['nickname']} ({node['model']})")
        print(f"    MAC: {node['mac']} | IP: {node['ip']}")
        print(f"    Status: {status_icon} {node['status']} | Role: {node['role']}")
        print(f"    Version: HW {node['hardware_version']} / SW {node['software_version']}")
        if node['signal_strength']:
            sig = node['signal_strength']
            print(f"    Signal: 2.4G={sig.get('band2_4', 'N/A')}dBm, 5G={sig.get('band5', 'N/A')}dBm")
        print()

    # Connected Devices
    print(f"[Connected Devices] ({len(data['devices'])} devices)")
    print("-" * 70)
    print(f"{'Hostname':<26} {'IP':<16} {'MAC':<18} {'Band':<8} {'Speed'}")
    print("-" * 70)
    for device in data['devices']:
        hostname = device['hostname'][:25] if len(device['hostname']) > 25 else device['hostname']
        speed = f"↓{device['down_speed']} ↑{device['up_speed']}" if device['down_speed'] or device['up_speed'] else "-"
        print(f"{hostname:<26} {device['ip']:<16} {device['mac']:<18} {device['connection']:<8} {speed}")
    print("-" * 70)


def main():
    parser = argparse.ArgumentParser(description='TP-Link DECO Router Monitor')
    parser.add_argument('--config', '-c',
                        default=os.path.join(os.path.dirname(__file__), 'config.json'),
                        help='Path to config.json (default: ./config.json)')
    parser.add_argument('--json', '-j', action='store_true',
                        help='Output in JSON format')
    parser.add_argument('--host', help='Override router IP address')
    parser.add_argument('--password', '-p', help='Override router password')

    args = parser.parse_args()

    try:
        # Load config
        config = load_config(args.config)

        # Allow command line overrides
        host = args.host or config.get('host', '192.168.68.1')
        password = args.password or config.get('password')

        if not password:
            print("Error: Password required. Set in config.json or use --password", file=sys.stderr)
            return 1

        data = get_deco_full_status(host, password)

        if args.json:
            print(json.dumps(data, indent=2, ensure_ascii=False))
        else:
            print_human_readable(data)

        return 0
    except FileNotFoundError:
        print(f"Error: Config file not found: {args.config}", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
