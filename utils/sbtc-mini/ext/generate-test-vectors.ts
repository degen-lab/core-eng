import {poxAddressToBtcAddress, poxAddressToTuple} from "https://esm.sh/@stacks/stacking";
import {cvToString} from "https://esm.sh/@stacks/transactions";
import { publicKeyToBtcAddress, getPublicKeyFromPrivate } from 'https://esm.sh/@stacks/encryption';

console.log("testv");
// wallet 1
console.log(cvToString(poxAddressToTuple("mr1iPkD9N3RJZZxXRk7xF9d36gffa6exNC")))
// wallet 2
console.log(cvToString(poxAddressToTuple("muYdXKmX9bByAueDe6KFfHd5Ff1gdN9ErG")))


// wallet 3
console.log(cvToString(poxAddressToTuple("mvZtbibDAAA3WLpY7zXXFqRa3T4XSknBX7")))
console.log(getPublicKeyFromPrivate("d655b2523bcd65e34889725c73064feb17ceb796831c0e111ba1a552b0f31b3901"))
// wallet 4
console.log(cvToString(poxAddressToTuple("mg1C76bNTutiCDV3t9nWhZs3Dc8LzUufj8")))
console.log(getPublicKeyFromPrivate("f9d7206a47f14d2870c163ebab4bf3e70d18f5d14ce1031f3902fbbc894fe4c701"))
