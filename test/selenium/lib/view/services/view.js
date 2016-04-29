var View = require('../view');
var ServicesAccessors = require('./accessors');


function ServicesView() {
  [].push.call(arguments, ServicesAccessors);
  View.apply(this, arguments);

  this.accessors.logOutButton;  // Wait until it appears
}

ServicesView.prototype = Object.assign({

 // To add functions here

}, View.prototype);
module.exports = ServicesView;
