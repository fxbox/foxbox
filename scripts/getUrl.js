var process = require('process');
console.log('http://ns.useraddress.net:5300/v1/dns/' + process.argv[2].split('.').reverse().join('/') + '/_acme-challenge');
