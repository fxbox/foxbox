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

    def getter(self, getter_key):
        getters = self.getters()
        if getter_key in getters:
            return getters[getter_key]

    def getter_contains(self, name):
        for (getter_key, getter) in self.getters().items():
            if name in getter_key:
                return (getter_key, getter)
        return (None, None)

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
        for (setter_key, setter) in self.setters().items():
            if name in setter_key:
                return (setter_key, setter)
        return (None, None)

    def setters(self):
        return self.service['setters']

    def fmt_response(self, getter_key, getter_req):
        if getter_req.headers['content-type'].startswith('application/json'):
            j = getter_req.json()
            if getter_key in j:
                rsp = j[getter_key]
                getter = self.getter(getter_key)
                for type in rsp:
                    if type == getter['kind']:
                        return rsp[type]

def main():
    default_server = 'localhost'
    default_port = 3000
    parser = argparse.ArgumentParser(
        prog="ipcam",
        usage="%(prog)s [options] [command]",
        description="Interact with Foxbox IP Cameras",
    )
    parser.add_argument(
        '-s', '--server',
        dest='server',
        default=default_server,
        help='Server to connect to (default is {})'.format(default_server),
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
        '--user',
        dest='username',
        action='store',
        help='Specify username for signing onto foxbox',
        default='admin'
    )
    parser.add_argument(
        '--password',
        dest='password',
        action='store',
        help='Specify password for signing onto foxbox',
        default=''
    )
    parser.add_argument(
        '--services',
        dest='services',
        action='store_true',
        help='List the available services',
    )
    parser.add_argument(
        '--service',
        dest='service',
        action='store',
        help='List the services which match',
    )
    parser.add_argument(
        '--service-property',
        dest='service_property',
        action='store',
        help='Filter services based on a property value',
    )
    parser.add_argument(
        '--get',
        dest='get',
        action='store',
        help='Retrieves the current value from the named getter',
    )
    parser.add_argument(
        '--set',
        dest='set',
        action='store',
        help='Sets the value of the named setter. (i.e. --set name=value)',
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
    login_url = '{}/users/login'.format(server_url)
    services_url = '{}/api/v1/services'.format(server_url)
    get_url = '{}/api/v1/channels/get'.format(server_url)
    set_url = '{}/api/v1/channels/set'.format(server_url)

    username = args.username
    password = args.password

    auth_filename = os.path.expanduser('~/.svc_auth_token')

    if args.verbose:
        print('server =', args.server)
        print('port =', args.port)
        print('service =', args.service)
        print('services =', args.services)
        print('service_property =', args.service_property)
        print('server_url =', server_url)
        print('login_url =', login_url)
        print('services_url =', services_url)
        print('get_url =', get_url)
        print('set_url =', set_url)
        print('username =', username)
        print('password =', password)
        print('get = ', args.get)
        print('set = ', args.set)

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
            try:
                r = requests.post(login_url, auth=(username, password))
            except requests.exceptions.ConnectionError:
                print('Unable to connect to server @ {}'.format(services_url))
                return
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
        try:
            r = requests.get(services_url, headers=auth_header)
        except requests.exceptions.ConnectionError:
            print('Unable to connect to server @ {}'.format(services_url))
            return
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

    for service in sorted(services, key=lambda entry: entry['adapter'] + entry['id']):
        svc = Service(service)
        if not svc.has_property_value(args.service_property):
            continue
        if args.services or args.service:
            if not args.service or args.service in svc.adapter():
                if args.verbose:
                    print(json.dumps(service, indent=4))
                else:
                    print('Adapter: {} ID: {}'.format(svc.adapter(), svc.id()))
                    print('  setters:')
                    for setter in sorted(svc.setters()):
                        print('    {}'.format(setter))
                    print('  getters:')
                    for getter in sorted(svc.getters()):
                        print('    {}'.format(getter))
        if args.get:
            getter_key, getter = svc.getter_contains(args.get);
            if not getter:
                continue
            getter_data = json.dumps({'id': getter_key})
            if args.verbose:
                print("Sending PUT to {} data={}".format(get_url, getter_data))
            getter_req = requests.put(get_url, headers=auth_header, data=bytes(getter_data, encoding='utf-8'))
            if args.verbose:
                print("Got {} response of '{}'".format(getter_req.headers['content-type'], getter_req.text))
            print("{} = '{}'".format(args.get, svc.fmt_response(getter_key, getter_req)))
        if args.set:
            set_name, set_value = args.set.split('=', 1)
            if args.verbose:
                print('set_name =', set_name)
                print('set_value =', set_value)
            setter_key, setter = svc.setter_contains(set_name);
            if not setter:
                continue
            setter_data = json.dumps({'select': {'id': setter_key}, 'value': {setter['kind']: set_value}})
            if args.verbose:
                print("Sending PUT to {} data={}".format(set_url, setter_data))
            setter_req = requests.put(set_url, headers=auth_header, data=bytes(setter_data, encoding='utf-8'))
            if args.verbose:
                print("Got {} response of '{}'".format(setter_req.headers['content-type'], setter_req.text))
            

if __name__ == "__main__":
    main()

