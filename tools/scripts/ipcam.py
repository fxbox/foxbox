#!/usr/bin/env python3
#
# Simple client for connecting with the IpCameraService.

import argparse
import getpass
import requests
import json
import os
import sys

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
        '--list-snaps',
        dest='list_snaps',
        action='store_true',
        help='List the snapshots available for a given IP Camera',
    )
    parser.add_argument(
        '--get',
        dest='get',
        action='store',
        help='Retrive a snapshot from the server.',
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
    services_url = '{}/services/list'.format(server_url)

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
        #print(service)
        if service['name'] == 'IpCameraService':
            service_id = service['id']
            service_descr = service['description']
            if args.name is None or args.name in service_descr:
                camera_found = True
                if args.list_cams or args.list_snaps:
                    print('id: {} description: {}'.format(service_id, service_descr))
                if args.snapshot:
                    snapshot_url = '{}/services/{}/snapshot'.format(server_url, service_id)
                    snapshot_req = requests.get(snapshot_url, headers=auth_header)
                    j_resp = snapshot_req.json()
                    print(j_resp['success'])
                if args.list_snaps:
                    list_snaps_url = '{}/services/{}/list'.format(server_url, service_id)
                    snaps_req = requests.get(list_snaps_url, headers=auth_header)
                    snaps = json.loads(str(snaps_req.content, 'utf-8'))
                    if snaps:
                        for snap in sorted(snaps):
                            print('    {}'.format(snap))
                    else:
                        print('    No snapshots available')
                if args.get:
                    filename = args.get
                    #print('get filename =', filename)
                    get_snap_url = '{}/services/{}/get?filename={}'.format(server_url, service_id, filename)
                    get_req = requests.get(get_snap_url, headers=auth_header)
                    if get_req.status_code == 200 and get_req.headers['content-type'] == 'image/jpeg':
                        with open(filename, 'wb') as f:
                            f.write(get_req.content)
                        print('Wrote image to {}'.format(filename))
                    else:
                        j_resp = get_req.json()
                        print(j_resp['error'])


    if not camera_found:
        if args.name is None:
            print('No IP Cameras found')
        else:
            print('No IP Cameras found with a description containing \'{}\''.format(args.name))



if __name__ == "__main__":
    main()

