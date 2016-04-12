/* global module */
'use strict';
var upnpServer = require('./upnpserver_base.js');

	class ipCamera extends upnpServer {

		constructor(ipValue,portValue,udnValue,usnValue,descriptionValue) {
			var cameraLocation = 'http://' + ipValue + ':' + portValue;
			var ipCamera_config = 
			{
				location:cameraLocation,
				udn:udnValue,
				description:descriptionValue
			};

			super(ipCamera_config);
			this._server.addUSN(usnValue);
		}

		startServer (something) {
			super.startServer(something);
		}
	}
module.exports = ipCamera;