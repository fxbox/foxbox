const https = require('https');
const fs = require('fs');

var args = process.argv;
var options = {
  hostname: args[2],
  port: args[3],
  path: args[4],
  method: 'POST',
  key: fs.readFileSync('certs/server/my-server.key.pem'),
  cert: fs.readFileSync('certs/server/my-server.crt.pem'),
  rejectUnauthorized: false
};
console.log(options);
var req = https.request(options, (res) => {
  console.log('statusCode: ', res.statusCode);
  console.log('headers: ', res.headers);

  res.on('data', (d) => {
    process.stdout.write(d);
  });
});

// write data to request body
req.write(args[5]);
req.end();

req.on('error', (e) => {
  console.error(e);
});
