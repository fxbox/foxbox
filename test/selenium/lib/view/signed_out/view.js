'use strict';

var View = require('../view.js');

function SignedOutPageView() {
  View.apply(this, arguments);

  this.accessors.root
}

SignedOutPageView.prototype = {
    hasSignedOut: function() {
        return this.accessors.root;
    }
};

module.exports = SignedOutPageView;
