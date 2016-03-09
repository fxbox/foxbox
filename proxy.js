#!/usr/bin/env node
'use strict';

var https = require('https');
var fs = require('fs');
var qr = require('qr-image');
var exec = require('child_process').exec;
var proxy = require('http-proxy').createProxyServer();
var mdns = require('mdns');
var ports = {
  backend: 3000,
  front: 4333
};
var os = require('os');
var TUNNEL_IPADDR = '52.36.71.23';

function run(cmd, ignoreStdErr) {
  console.log(cmd);
  return new Promise((resolve, reject) => {
    exec(cmd, (err, stdout, stderr) => {
      if (err) {
        reject(err);
      }
      if (stderr.length) {
        console.log('stderr detected:', cmd, stdout, stderr);
        if (!ignoreStdErr) {
          reject(stderr);
        }
      }
      resolve(stdout);
    });
  });
}

function buildSelfSigned() {
  let fqdn;
  // Make directories to work from
  return run('mkdir -p certs/{server,client,ca,tmp}').then(() => {
    // Create your very own Root Certificate Authority
    return run('openssl genrsa -out certs/ca/my-root-ca.key.pem 2048', true);
  }).then(() => {
    // Self-sign your Root Certificate Authority
    // Since this is private, the details can be as bogus as you like
    return run('openssl req -x509 -new -nodes -key certs/ca/my-root-ca.key.pem ' +
        '-days 1000000 -out certs/ca/my-root-ca.crt.pem ' +
        '-subj "/C=US/ST=Utah/L=Provo/O=ACME Signing Authority Inc/CN=example.com"');
  }).then(() => {
    // Create a Device Certificate for each domain,
    // such as example.com, *.example.com, awesome.example.com
    // NOTE: You MUST match CN to the domain name or ip address you want to use
    return run('openssl genrsa -out certs/server/my-server.key.pem 2048', true);
  }).then(() => {
    // Determine the fingerprint of the signing cert
    return run('openssl x509 -in certs/ca/my-root-ca.crt.pem -sha256 -noout -fingerprint');
  }).then(out => {
    fqdn = out.substring('SHA256 Fingerprint='.length)
        .split(':').join('').toLowerCase().trim().substring(0, 32) + '.self-signed';
    // Create a request from your Device, which your Root CA will sign
    return run('openssl req -new -key certs/server/my-server.key.pem ' +
      '-out certs/tmp/my-server.csr.pem ' +
      `-subj "/C=US/ST=Utah/L=Provo/O=ACME Tech Inc/CN=${fqdn}"`, true);
  }).then(() => {
    // Sign the request from Device with your Root CA
    // -CAserial certs/ca/my-root-ca.srl
    return run('openssl x509 -req -in certs/tmp/my-server.csr.pem ' +
        '-CA certs/ca/my-root-ca.crt.pem ' +
        '-CAkey certs/ca/my-root-ca.key.pem ' +
        '-CAcreateserial ' +
        '-out certs/server/my-server.crt.pem ' +
        '-days 1000000', true);
  }).then(() => {
    console.log(`Generated certificate chain for ${fqdn} in ./certs.`);
    return fqdn;
  });
}

function getLocalIPAddr() {
  return new Promise((resolve, reject) => {
    var ifaces = os.networkInterfaces();
    Object.keys(ifaces).forEach(function (ifname) {
      var alias = 0;

      ifaces[ifname].forEach(function (iface) {
        if ('IPv4' !== iface.family || iface.internal !== false) {
          // skip over internal (i.e. 127.0.0.1) and non-ipv4 addresses
          return;
        }
        resolve(iface.address);
        // if (alias >= 1) {
        //   // this single interface has multiple ipv4 addresses
        //   console.log(ifname + ':' + alias, iface.address);
        // } else {
        //   // this interface has only one ipv4 adress
        //   console.log(ifname, iface.address);
        // }
        // ++alias;
      });
    });
    reject(new Error('Local IP address not found'));
  });
}

function buildPublicLocal(fqdn) {
  var hash = fqdn.split('.')[0];
  return getLocalIPAddr().then(localIpAddr => {
    var cmd = `cd scripts ; ./update.sh ${hash} ${localIpAddr} ${TUNNEL_IPADDR}`;
    console.log(`Running ${cmd}`);
    return run(cmd);
  }).then(stdout => {
    console.log(stdout);
  });
}

function mdnsServe(fqdn) {
  // advertise a https server:
  mdns.createAdvertisement(mdns.tcp('https'), ports.front, {
    // seems that https://www.npmjs.com/package/cordova-plugin-zeroconf does not
    // support custom name field, so using txtRecord instead:
    txtRecord: {
      name: fqdn
    }
  }).start();

  // // For debugging purposes:
  // var browser = mdns.createBrowser(mdns.tcp('https'));
  // browser.on('serviceUp', function(service) {
  //   console.log("service up: ", service);
  // });
  // browser.on('serviceDown', function(service) {
  //   console.log("service down: ", service);
  // });
  // browser.start();
  return Promise.resolve();
}

function qrGen(fqdn) {
  const qrCodeString = `https://${fqdn}:${ports.front}/`;
  const qr_svg = qr.image(qrCodeString, { type: 'svg' });
  qr_svg.pipe(fs.createWriteStream('qr.svg'));
  console.log(`Wrote string ${qrCodeString} into ./qr.svg, please display and scan.`);
  return Promise.resolve();
}

function proxyServe(fqdn) {
  // serve a web server on the local network:
  https.createServer({
    key: fs.readFileSync('certs/server/my-server.key.pem'),
    cert: fs.readFileSync('certs/server/my-server.crt.pem'),
    ca: fs.readFileSync('certs/ca/my-root-ca.crt.pem')
  }, (req, res) => {
    proxy.web(req, res, { target: `http://localhost:${ports.backend}` });
  }).listen(ports.front);
  console.log(`Proxying https port ${ports.front} to http port ${ports.backend}, ` +
      `ready for connections.`);
  return Promise.resolve();
}

//...
buildSelfSigned().then(fqdn => {
  return buildPublicLocal(fqdn).then(stdout => {
    console.log(stdout);
    return mdnsServe(fqdn);
  }).then(() => {
    return qrGen(fqdn);
  }).then(() => {
    return proxyServe(fqdn);
  });
}).catch(err => {
  console.error(err);
});
