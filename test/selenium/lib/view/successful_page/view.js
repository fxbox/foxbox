'use strict';

var SuccessfulPageAccessor = require('./accessors.js');
var View = require('../view');

function SuccessfulPageView() {
  [].push.call(arguments, SuccessfulPageAccessor);
  View.apply(this, arguments);

  this.accessors.successMessageLocator; // Wait until message is displayed
}

SuccessfulPageView.prototype = Object.assign({
  get loginMessage() {
    return this.accessors.successMessageLocator.getText();
  },

  goToSignedIn() {
    return this.driver.navigate().to('http://localhost:3331')
    .then(() => this.instanciateNextView('signed_in'));
  }
}, View.prototype);

module.exports = SuccessfulPageView;
