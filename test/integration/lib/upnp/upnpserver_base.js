/* global module */
'use strict';

const Server = require('node-ssdp').Server;

class upnpServer {
  constructor(parameters) {
    this.parameters = parameters;
    this._server = new Server(parameters);
  }

  createServer() {
    console.log('Defining callbacks');
    this._server.on('advertise-alive', function (headers) {
      // Expire old devices from your cache. 
      // Register advertising device somewhere 
      // (as designated in http headers heads) 
      console.log('Advertising');
      console.log(headers);
    });

    this._server.on('advertise-bye', function (headers) {
      // Remove specified device from cache. 
      console.log('Saying Bye');
      console.log(headers);
    });    
  }

  startServer(something) {
    console.log('Start server: ' + something);
    this._server.start();      
  }

  stopServer(something) {   
    this._server.stop();   
    console.log('Stop server: ' + something);   
  }
}
       
module.exports = upnpServer;
    