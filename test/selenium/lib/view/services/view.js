'use strict';

var View = require('../view');


function ServicesView() {
  View.apply(this, arguments);

  this.accessors.logOutButton;  // Wait until it appears
}

ServicesView.prototype = Object.assign({

 // To add functions here

}, View.prototype);
module.exports = ServicesView;
