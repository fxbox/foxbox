'use strict';

var View = require('../view.js')


function SignedInPageView() {
  View.apply(this, arguments);

  this.accessor.signOutButton; // Wait until button is displayed
}

SignedInPageView.prototype = Object.assign({
  signOut: function() {
    return this.accessor.signOutButton.click()
    .then(() => this.instanciateNextView('signed_out'));
  }
}, View.prototype);

module.exports = SignedInPageView;
