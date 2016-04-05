'use strict';

const express = require('express');
const path = require('path');
const Config = require('config-js');

var upnp_ipcamera = require('./upnp/upnp_ipcamera.js');
var config = new Config('./test/integration/lib/config/foxbox.js');

var ipCamera_server = (function() {

  // start a webserver that hosts the xml
  var instance;
  var ip = config.get('ipCamera.ip');
  var port = config.get('ipCamera.port');
  var udn = config.get('ipCamera.udn');
  var usn = config.get('ipCamera.usn');
  var desc = config.get('ipCamera.description');
  var cameraUPnPServer = new upnp_ipcamera(ip,port,udn,usn,desc);

  function setup() {
    var _server = express();

    _server.get('/', function (req, res) {
      res.status(200).type('xml').send(
       '<?xml version=\"1.0\"?>' +
       '<root xmlns=\"urn:schemas-upnp-org:device-1-0\">' +
       '<specVersion>' + 
       '<major>1</major>' +
       '<minor>0</minor>' + 
       '</specVersion>' +
       '<URLBase>http://' + ip + ':' + port + '</URLBase>' +
       '<device>' +
       '<deviceType>urn:schemas-upnp-org:device:Basic:1.0</deviceType>' +
       '<friendlyName>Link IpCam(' + ip + ':' + port + ')</friendlyName>' +
       '<manufacturer>Project-Link</manufacturer>' +
       '<modelDescription>Wireless Internet Camera</modelDescription>' +
       '<modelName>Link-IpCamera</modelName>' +
       '<modelNumber>Link-IpCamera</modelNumber>' +
       '<UDN>' + udn + '</UDN>' +
       '<UPC/>' +
       '<serviceList>' +
       '<service>' +
       '<serviceType>' + usn + '</serviceType>' +
       '<serviceId>urn:cellvision:serviceId:RootNull</serviceId>' +
       '<SCPDURL>/rootService.xml</SCPDURL>' +
       '</service>' +
       '</serviceList>' +
       '<presentationURL>http://' + ip + ':' + port + '</presentationURL>' +
       '</device>' +
       '</root>'
      );
    });
    
    _server.get('/image/jpeg.cgi', function (req, res) {
        //send a jpeg
        console.log('Camera: snapshot request received');
        res.status(200).sendFile(
          path.join(__dirname,'colville.jpg'));
    });

    instance = _server.listen(parseInt(port), function () {
      // start the upnp broadcast
      cameraUPnPServer.startServer('Dlink IP Camera');
    });
  }

  function stop() {
    return new Promise(resolve => {
      // Stopping cameraUPnPServer causes socket error on next script. disabled.
      // Also, Wireshark does not show additional traffic even when not stopped.
     instance.close(function(){
        console.log('Dlink IP Camera off');
       resolve();
      });
    });
  }

return {setup,stop};

})();

module.exports = ipCamera_server;
