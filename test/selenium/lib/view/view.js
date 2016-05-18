'use strict';

function View(driver, Accessors) {
  this.driver = driver;
  this.accessors = new Accessors(driver);
}

View.prototype = {
  instanciateNextView(viewFolderName) {
    const NextView = require('./' + viewFolderName + '/view.js');
    return new NextView(this.driver);
  }
};

module.exports = View;
