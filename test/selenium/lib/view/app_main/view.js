'use strict';

var View = require('../view');


function MainView() {
  View.apply(this, arguments);

  this.accessor.connectToFoxBoxButton;
}

MainView.prototype = Object.assign({

  connectToFoxBox: function() {
    return this.accessor.connectToFoxBoxButton.click()
    .then(() => this.instanciateNextView('sign_up'));
  }

}, View.prototype);

module.exports = MainView;
