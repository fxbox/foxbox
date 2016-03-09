'use strict';

var express = require ('express');

var philips_nupnp_page = (function() {
  return {
    start: function(hue_id,hue_ipaddress,port) {
      var app = express();

      app.get('/', function (req, res) {
        res.send([{'id':hue_id,'internalipaddress':hue_ipaddress}]);
      });
      
      app.listen(port, function () {
        console.log('Philips nupnp app listening on port ' + port);
      });
    }
  };
})();

module.exports = philips_nupnp_page;