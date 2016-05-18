'use strict';

var View = require('../view.js')


function SignedInPageView() {
  View.apply(this, arguments);

  this.accessors.signOutButton; // Wait until button is displayed
}

SignedInPageView.prototype = Object.assign({
  signOut: function() {
    return this.accessors.signOutButton.click()
    .then(() => this.instanciateNextView('signed_out'));
  }
}, View.prototype);

module.exports = SignedInPageView;
