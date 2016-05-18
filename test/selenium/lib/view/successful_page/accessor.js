'use strict';

var Accessor = require('../accessor');


function SuccessfulPageAccessor() {
  Accessor.apply(this, arguments);
}

SuccessfulPageAccessor.prototype = Object.assign({
  get successMessageLocator() {
    return this.waitForElement('#thank-you');
  }
}, Accessor.prototype);

module.exports = SuccessfulPageAccessor;
