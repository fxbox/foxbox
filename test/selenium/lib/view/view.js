'use strict';

const stack = require('callsite');
const path = require('path');

function View(driver) {
  this.driver = driver;

  // Here we fetch `accessors.js` located in the same directory than
  // the child view
  var lastCallerInStackStace = stack()[1]; // 0 actually points to this line
  var childViewDirectory = path.dirname(lastCallerInStackStace.getFileName());
  const ChildAccessors = require(childViewDirectory + '/accessors.js');
  this.accessors = new ChildAccessors(this.driver);
}

View.prototype = {
  instanciateNextView(viewFolderName) {
    const NextView = require('./' + viewFolderName + '/view.js');
    return new NextView(this.driver);
  }
};

module.exports = View;
