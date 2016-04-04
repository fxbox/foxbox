#!/usr/bin/env python3
#
# Simple client for connecting with the IpCameraService.
#

import argparse
import getpass
import requests
import json
import os
import sys

class Service:

    def __init__(self, service):
        self.service = service

    def adapter(self):
        return self.service['adapter']

    def id(self):
        return self.service['id']

    def is_adapter(self, adapter_name):
        return self.service['adapter'].startswith(adapter_name)

    def getter_contains(self, name):
        getter_name = 'getter:' + name + '.'
        for key in self.service['getters']:
            if key.startswith(getter_name):
                return key
        print("Unable to find getter for '{}'".format(name))

    def getters(self):
        return self.service['getters']

    def property(self, name):
        if name in self.service['properties']:
            return self.service['properties'][name]

    def has_properties(self):
        return 'properties' in self.service

    def has_property_value(self, value):
        if value is None:
            return True
        if not self.has_properties():
            return False
        name = self.property('name')
        return name and value in name

    def setter_contains(self, name):
        setter_name = 'setter:' + name + '.'
        for key in self.service['setters']:
            if key.startswith(setter_name):
                return key
        print("Unable to find setter for '{}'".format(name))

    def setters(self):
        return self.service['setters']


def main():
    default_server = 'localhost'
    default_port = 3000
    parser = argparse.ArgumentParser(
        prog="ipcam",
        usage="%(prog)s [options] [command]",
        description="Interact with Foxbox IP Cameras",
    )
    parser.add_argument(
        '-l', '--list-cams',
        dest='list_cams',
        action='store_true',
        help='List the available IP Cameras',
    )
    parser.add_argument(
        '--list-services',
        dest='list_services',
        action='store_true',
        help='List the available services',
    )
    parser.add_argument(
        '--list-snaps',
        dest='list_snaps',
        action='store_true',
        help='List the snapshots available for a given IP Camera',
    )
    parser.add_argument(
        '--get',
        dest='get',
        action='store_true',
        help='Retrieve the latest snapshot from the server.',
    )
    parser.add_argument(
        '--snapshot',
        dest='snapshot',
        action='store_true',
        help='Take a snapshot',
        default=False
    )
    parser.add_argument(
        '-s', '--server',
        dest='server',
        default=default_server,
        help='Server to connect to (default is {})'.format(default_server),
    )
    parser.add_argument(
        '--password',
        dest='password',
        action='store',
        help='Specify password for signing onto foxbox',
        default=''
    )
    parser.add_argument(
        '-p', '--port',
        dest='port',
        action='store',
        type=int,
        default=default_port,
        help='Port to connect to (default is {})'.format(default_port),
    )
    parser.add_argument(
        '-n', '-name',
        dest='name',
        action='store',
        help='Portion of camera name to serach for',
    )
    parser.add_argument(
        '--user',
        dest='username',
        action='store',
        help='Specify username for signing onto foxbox',
        default='admin'
    )
    parser.add_argument(
        '-v', '--verbose',
        dest='verbose',
        action='store_true',
        help='Turn on verbose messages',
        default=False
    )
    args = parser.parse_args(sys.argv[1:])

    server_url = 'http://{}:{}'.format(args.server, args.port)
    services_url = '{}/api/v1/services'.format(server_url)
    get_url = '{}/api/v1/channels/get'.format(server_url)
    set_url = '{}/api/v1/channels/set'.format(server_url)

    username = args.username
    password = args.password

    auth_filename = os.path.expanduser('~/.ipcam_auth_token')

    if args.verbose:
        print('server =', args.server)
        print('port =', args.port)
        print('name =', args.name)
        print('server_url =', server_url)
        print('services_url =', services_url)
        print('username =', username)
        print('password =', password)

    token = None
    token_changed = False
    if not password:
        try:
            with open(auth_filename, 'rt') as f:
                token = f.read()
        except:
            # Unable to read token. This means that a password must be provided
            pass
    while True:
        if not password and not token:
            # User didn't provide a password as an argument, or it was invalid
            # prompt the user for a password
            password = getpass.getpass(prompt='Enter password for {} user: '.format(username))
        if password:
            # if a password was provided - use it, even if we had stashed a token
            login_url = '{}/users/login'.format(server_url)
            r = requests.post(login_url, auth=(username, password))
            if r.status_code != 201:
                print('Authentication failed')
                password = None
                if args.verbose:
                    print('Status Code:', r.status_code)
                    print('Headers:', r.headers)
                    print('Content:', r.content)
                continue

            # login was successful
            j_resp = json.loads(str(r.content, 'utf-8'))
            token = j_resp['session_token']
            token_changed = True

        # We now have a token - try it out
        auth_header = {'Authorization': 'Bearer {}'.format(token)}
        r = requests.get(services_url, headers=auth_header)
        if r.status_code == 200:
            # Token was accepted
            break
        print('Login failed')
        if args.verbose:
            print('Unable to get service list from {} ({})'.format(server_url, r.status_code))
            print(str(r.content, 'utf-8'))
        token = None
        password = None

    if token_changed:
        # Persist the token
        print('Saving authentication token')
        with open(auth_filename, 'wt') as f:
            f.write(token)

    services = json.loads(str(r.content, 'utf-8'))

    camera_found = False
    for service in services:
        if args.list_services:
            print(json.dumps(service, indent=4))
        svc = Service(service)
        if svc.is_adapter('ip-camera'):
            service_id = service['id'].replace('service:', '')
            if args.verbose: print('service_id =', service_id)
            camera_name = svc.property('name')
            if args.name is None or args.name in camera_name:
                camera_found = True
                if args.list_cams or args.list_snaps:
                    print('id: {} name: {}'.format(service_id, camera_name))
                if args.snapshot:
                    setter = svc.setter_contains('snapshot');
                    if setter:
                        setter_data = json.dumps({'select': {'id': setter}, 'value': {'Unit': []}})
                        if args.verbose:
                            print(setter_data)
                        setter_req = requests.put(set_url, headers=auth_header, data=bytes(setter_data, encoding='utf-8'))
                        if args.verbose:
                            print("Got {} response of '{}'".format(setter_req.headers['content-type'], setter_req.text))
                        print("Took a snapshot")
                if args.list_snaps:
                    getter = svc.getter_contains('image_list')
                    getter_data = json.dumps({'id': getter})
                    if args.verbose:
                        print(getter_data)
                    getter_req = requests.put(get_url, headers=auth_header, data=bytes(getter_data, encoding='utf-8'))
                    if args.verbose:
                        print(getter_req.text)
                    snaps_json = getter_req.json()
                    snaps = snaps_json[getter]['Json']
                    if snaps:
                        for snap in sorted(snaps):
                            print('    {}'.format(snap))
                    else:
                        print('    No snapshots available')
                if args.get:
                    getter = svc.getter_contains('image_newest')
                    getter_data = json.dumps({'id': getter})
                    if args.verbose:
                        print(getter_data)
                    getter_req = requests.put(get_url, headers=auth_header, data=bytes(getter_data, encoding='utf-8'))
                    if getter_req.status_code == 200 and getter_req.headers['content-type'] == 'image/jpeg':
                        filename = 'image.jpg'
                        with open(filename, 'wb') as f:
                            f.write(getter_req.content)
                        print('Wrote image to {}'.format(filename))
                    else:
                        j_resp = getter_req.json()
                        print(json.dumps(j_resp, indent=4))
    if not camera_found:
        if args.name is None:
            print('No IP Cameras found')
        else:
            print('No IP Cameras found with a description containing \'{}\''.format(args.name))


if __name__ == "__main__":
    main()

