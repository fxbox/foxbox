'use strict';
var express = require ('express');

var philips_nupnp_page = (function() {
  var _app = express();
  var instance;
  
  function start(hue_id,hue_ipaddress,port) {

    _app.get('/', function (req, res) {
      res.send([{'id':hue_id,'internalipaddress':hue_ipaddress}]);
    });
    
    return new Promise(resolve => {
     instance = _app.listen(port, function () {
      console.log('Philips nupnp app listening on port ' + port);
      resolve();
    });
   });
  }

  function stop() {
   return new Promise(resolve => {
     instance.close(function(){
       console.log('nupnp server closed');
           resolve(); // it's like if you called `callback()`
         });
   });
 }

 return {start,stop};
})();

module.exports = philips_nupnp_page;