'use strict';

var View = require('../view.js');

function SignedOutPageView() {
  View.apply(this, arguments);

  this.accessor.root
}

SignedOutPageView.prototype = {
    hasSignedOut: function() {
        return this.accessor.root;
    }
};

module.exports = SignedOutPageView;
