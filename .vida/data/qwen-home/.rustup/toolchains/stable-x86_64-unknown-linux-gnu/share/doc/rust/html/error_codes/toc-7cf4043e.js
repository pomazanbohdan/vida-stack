// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="error-index.html">Rust error codes index</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0001.html">E0001</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0002.html">E0002</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0004.html">E0004</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0005.html">E0005</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0007.html">E0007</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0009.html">E0009</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0010.html">E0010</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0013.html">E0013</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0014.html">E0014</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0015.html">E0015</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0023.html">E0023</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0025.html">E0025</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0026.html">E0026</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0027.html">E0027</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0029.html">E0029</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0030.html">E0030</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0033.html">E0033</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0034.html">E0034</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0038.html">E0038</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0040.html">E0040</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0044.html">E0044</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0045.html">E0045</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0046.html">E0046</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0049.html">E0049</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0050.html">E0050</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0053.html">E0053</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0054.html">E0054</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0055.html">E0055</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0057.html">E0057</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0059.html">E0059</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0060.html">E0060</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0061.html">E0061</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0062.html">E0062</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0063.html">E0063</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0067.html">E0067</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0069.html">E0069</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0070.html">E0070</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0071.html">E0071</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0072.html">E0072</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0073.html">E0073</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0074.html">E0074</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0075.html">E0075</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0076.html">E0076</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0077.html">E0077</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0080.html">E0080</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0081.html">E0081</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0084.html">E0084</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0087.html">E0087</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0088.html">E0088</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0089.html">E0089</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0090.html">E0090</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0091.html">E0091</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0092.html">E0092</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0093.html">E0093</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0094.html">E0094</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0106.html">E0106</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0107.html">E0107</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0109.html">E0109</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0110.html">E0110</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0116.html">E0116</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0117.html">E0117</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0118.html">E0118</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0119.html">E0119</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0120.html">E0120</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0121.html">E0121</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0124.html">E0124</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0128.html">E0128</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0130.html">E0130</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0131.html">E0131</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0132.html">E0132</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0133.html">E0133</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0136.html">E0136</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0137.html">E0137</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0138.html">E0138</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0139.html">E0139</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0152.html">E0152</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0154.html">E0154</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0158.html">E0158</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0161.html">E0161</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0162.html">E0162</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0164.html">E0164</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0165.html">E0165</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0170.html">E0170</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0178.html">E0178</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0183.html">E0183</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0184.html">E0184</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0185.html">E0185</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0186.html">E0186</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0191.html">E0191</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0192.html">E0192</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0193.html">E0193</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0195.html">E0195</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0197.html">E0197</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0198.html">E0198</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0199.html">E0199</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0200.html">E0200</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0201.html">E0201</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0203.html">E0203</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0204.html">E0204</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0205.html">E0205</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0206.html">E0206</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0207.html">E0207</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0208.html">E0208</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0210.html">E0210</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0211.html">E0211</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0212.html">E0212</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0214.html">E0214</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0220.html">E0220</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0221.html">E0221</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0222.html">E0222</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0223.html">E0223</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0224.html">E0224</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0225.html">E0225</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0226.html">E0226</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0227.html">E0227</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0228.html">E0228</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0229.html">E0229</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0230.html">E0230</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0231.html">E0231</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0232.html">E0232</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0243.html">E0243</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0244.html">E0244</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0251.html">E0251</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0252.html">E0252</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0253.html">E0253</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0254.html">E0254</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0255.html">E0255</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0256.html">E0256</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0259.html">E0259</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0260.html">E0260</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0261.html">E0261</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0262.html">E0262</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0263.html">E0263</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0264.html">E0264</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0267.html">E0267</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0268.html">E0268</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0271.html">E0271</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0275.html">E0275</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0276.html">E0276</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0277.html">E0277</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0281.html">E0281</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0282.html">E0282</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0283.html">E0283</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0284.html">E0284</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0297.html">E0297</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0301.html">E0301</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0302.html">E0302</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0303.html">E0303</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0307.html">E0307</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0308.html">E0308</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0309.html">E0309</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0310.html">E0310</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0311.html">E0311</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0312.html">E0312</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0316.html">E0316</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0317.html">E0317</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0320.html">E0320</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0321.html">E0321</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0322.html">E0322</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0323.html">E0323</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0324.html">E0324</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0325.html">E0325</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0326.html">E0326</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0328.html">E0328</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0329.html">E0329</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0364.html">E0364</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0365.html">E0365</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0366.html">E0366</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0367.html">E0367</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0368.html">E0368</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0369.html">E0369</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0370.html">E0370</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0371.html">E0371</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0373.html">E0373</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0374.html">E0374</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0375.html">E0375</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0376.html">E0376</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0377.html">E0377</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0378.html">E0378</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0379.html">E0379</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0380.html">E0380</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0381.html">E0381</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0382.html">E0382</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0383.html">E0383</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0384.html">E0384</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0386.html">E0386</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0387.html">E0387</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0388.html">E0388</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0389.html">E0389</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0390.html">E0390</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0391.html">E0391</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0392.html">E0392</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0393.html">E0393</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0398.html">E0398</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0399.html">E0399</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0401.html">E0401</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0403.html">E0403</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0404.html">E0404</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0405.html">E0405</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0407.html">E0407</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0408.html">E0408</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0409.html">E0409</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0411.html">E0411</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0412.html">E0412</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0415.html">E0415</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0416.html">E0416</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0422.html">E0422</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0423.html">E0423</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0424.html">E0424</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0425.html">E0425</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0426.html">E0426</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0428.html">E0428</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0429.html">E0429</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0430.html">E0430</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0431.html">E0431</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0432.html">E0432</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0433.html">E0433</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0434.html">E0434</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0435.html">E0435</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0436.html">E0436</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0437.html">E0437</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0438.html">E0438</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0439.html">E0439</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0445.html">E0445</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0446.html">E0446</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0447.html">E0447</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0448.html">E0448</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0449.html">E0449</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0451.html">E0451</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0452.html">E0452</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0453.html">E0453</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0454.html">E0454</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0455.html">E0455</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0457.html">E0457</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0458.html">E0458</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0459.html">E0459</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0460.html">E0460</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0461.html">E0461</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0462.html">E0462</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0463.html">E0463</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0464.html">E0464</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0466.html">E0466</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0468.html">E0468</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0469.html">E0469</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0472.html">E0472</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0476.html">E0476</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0477.html">E0477</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0478.html">E0478</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0482.html">E0482</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0491.html">E0491</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0492.html">E0492</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0493.html">E0493</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0495.html">E0495</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0496.html">E0496</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0497.html">E0497</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0498.html">E0498</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0499.html">E0499</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0500.html">E0500</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0501.html">E0501</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0502.html">E0502</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0503.html">E0503</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0504.html">E0504</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0505.html">E0505</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0506.html">E0506</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0507.html">E0507</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0508.html">E0508</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0509.html">E0509</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0510.html">E0510</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0511.html">E0511</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0512.html">E0512</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0514.html">E0514</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0515.html">E0515</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0516.html">E0516</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0517.html">E0517</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0518.html">E0518</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0519.html">E0519</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0520.html">E0520</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0521.html">E0521</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0522.html">E0522</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0523.html">E0523</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0524.html">E0524</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0525.html">E0525</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0527.html">E0527</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0528.html">E0528</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0529.html">E0529</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0530.html">E0530</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0531.html">E0531</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0532.html">E0532</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0533.html">E0533</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0534.html">E0534</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0535.html">E0535</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0536.html">E0536</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0537.html">E0537</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0538.html">E0538</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0539.html">E0539</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0541.html">E0541</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0542.html">E0542</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0543.html">E0543</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0544.html">E0544</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0545.html">E0545</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0546.html">E0546</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0547.html">E0547</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0549.html">E0549</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0550.html">E0550</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0551.html">E0551</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0552.html">E0552</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0554.html">E0554</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0556.html">E0556</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0557.html">E0557</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0559.html">E0559</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0560.html">E0560</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0561.html">E0561</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0562.html">E0562</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0565.html">E0565</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0566.html">E0566</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0567.html">E0567</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0568.html">E0568</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0569.html">E0569</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0570.html">E0570</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0571.html">E0571</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0572.html">E0572</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0573.html">E0573</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0574.html">E0574</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0575.html">E0575</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0576.html">E0576</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0577.html">E0577</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0578.html">E0578</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0579.html">E0579</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0580.html">E0580</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0581.html">E0581</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0582.html">E0582</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0583.html">E0583</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0584.html">E0584</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0585.html">E0585</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0586.html">E0586</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0587.html">E0587</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0588.html">E0588</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0589.html">E0589</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0590.html">E0590</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0591.html">E0591</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0592.html">E0592</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0593.html">E0593</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0594.html">E0594</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0595.html">E0595</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0596.html">E0596</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0597.html">E0597</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0599.html">E0599</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0600.html">E0600</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0601.html">E0601</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0602.html">E0602</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0603.html">E0603</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0604.html">E0604</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0605.html">E0605</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0606.html">E0606</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0607.html">E0607</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0608.html">E0608</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0609.html">E0609</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0610.html">E0610</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0614.html">E0614</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0615.html">E0615</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0616.html">E0616</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0617.html">E0617</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0618.html">E0618</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0619.html">E0619</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0620.html">E0620</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0621.html">E0621</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0622.html">E0622</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0623.html">E0623</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0624.html">E0624</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0625.html">E0625</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0626.html">E0626</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0627.html">E0627</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0628.html">E0628</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0631.html">E0631</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0632.html">E0632</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0633.html">E0633</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0634.html">E0634</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0635.html">E0635</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0636.html">E0636</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0637.html">E0637</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0638.html">E0638</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0639.html">E0639</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0640.html">E0640</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0641.html">E0641</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0642.html">E0642</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0643.html">E0643</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0644.html">E0644</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0646.html">E0646</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0647.html">E0647</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0648.html">E0648</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0657.html">E0657</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0658.html">E0658</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0659.html">E0659</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0660.html">E0660</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0661.html">E0661</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0662.html">E0662</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0663.html">E0663</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0664.html">E0664</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0665.html">E0665</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0666.html">E0666</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0667.html">E0667</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0668.html">E0668</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0669.html">E0669</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0670.html">E0670</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0671.html">E0671</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0687.html">E0687</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0688.html">E0688</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0689.html">E0689</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0690.html">E0690</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0691.html">E0691</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0692.html">E0692</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0693.html">E0693</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0695.html">E0695</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0696.html">E0696</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0697.html">E0697</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0698.html">E0698</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0699.html">E0699</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0700.html">E0700</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0701.html">E0701</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0703.html">E0703</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0704.html">E0704</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0705.html">E0705</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0706.html">E0706</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0708.html">E0708</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0710.html">E0710</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0712.html">E0712</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0713.html">E0713</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0714.html">E0714</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0715.html">E0715</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0716.html">E0716</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0711.html">E0711</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0717.html">E0717</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0718.html">E0718</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0719.html">E0719</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0720.html">E0720</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0722.html">E0722</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0724.html">E0724</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0725.html">E0725</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0726.html">E0726</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0727.html">E0727</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0728.html">E0728</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0729.html">E0729</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0730.html">E0730</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0731.html">E0731</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0732.html">E0732</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0733.html">E0733</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0734.html">E0734</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0735.html">E0735</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0736.html">E0736</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0737.html">E0737</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0739.html">E0739</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0740.html">E0740</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0741.html">E0741</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0742.html">E0742</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0743.html">E0743</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0744.html">E0744</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0745.html">E0745</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0746.html">E0746</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0747.html">E0747</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0748.html">E0748</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0749.html">E0749</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0750.html">E0750</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0751.html">E0751</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0752.html">E0752</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0753.html">E0753</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0754.html">E0754</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0755.html">E0755</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0756.html">E0756</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0757.html">E0757</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0758.html">E0758</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0759.html">E0759</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0760.html">E0760</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0761.html">E0761</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0762.html">E0762</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0763.html">E0763</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0764.html">E0764</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0765.html">E0765</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0766.html">E0766</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0767.html">E0767</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0768.html">E0768</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0769.html">E0769</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0770.html">E0770</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0771.html">E0771</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0772.html">E0772</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0773.html">E0773</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0774.html">E0774</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0775.html">E0775</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0776.html">E0776</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0777.html">E0777</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0778.html">E0778</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0779.html">E0779</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0780.html">E0780</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0781.html">E0781</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0782.html">E0782</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0783.html">E0783</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0784.html">E0784</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0785.html">E0785</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0786.html">E0786</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0787.html">E0787</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0788.html">E0788</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0789.html">E0789</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0790.html">E0790</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0791.html">E0791</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0792.html">E0792</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0793.html">E0793</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0794.html">E0794</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0795.html">E0795</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0796.html">E0796</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0797.html">E0797</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0798.html">E0798</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0799.html">E0799</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0800.html">E0800</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0801.html">E0801</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0802.html">E0802</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0803.html">E0803</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0804.html">E0804</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0805.html">E0805</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="E0806.html">E0806</a></span></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split('#')[0].split('?')[0];
        if (current_page.endsWith('/')) {
            current_page += 'index.html';
        }
        const links = Array.prototype.slice.call(this.querySelectorAll('a'));
        const l = links.length;
        for (let i = 0; i < l; ++i) {
            const link = links[i];
            const href = link.getAttribute('href');
            if (href && !href.startsWith('#') && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The 'index' page is supposed to alias the first chapter in the book.
            if (link.href === current_page
                || i === 0
                && path_to_root === ''
                && current_page.endsWith('/index.html')) {
                link.classList.add('active');
                let parent = link.parentElement;
                while (parent) {
                    if (parent.tagName === 'LI' && parent.classList.contains('chapter-item')) {
                        parent.classList.add('expanded');
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', e => {
            if (e.target.tagName === 'A') {
                const clientRect = e.target.getBoundingClientRect();
                const sidebarRect = this.getBoundingClientRect();
                sessionStorage.setItem('sidebar-scroll-offset', clientRect.top - sidebarRect.top);
            }
        }, { passive: true });
        const sidebarScrollOffset = sessionStorage.getItem('sidebar-scroll-offset');
        sessionStorage.removeItem('sidebar-scroll-offset');
        if (sidebarScrollOffset !== null) {
            // preserve sidebar scroll position when navigating via links within sidebar
            const activeSection = this.querySelector('.active');
            if (activeSection) {
                const clientRect = activeSection.getBoundingClientRect();
                const sidebarRect = this.getBoundingClientRect();
                const currentOffset = clientRect.top - sidebarRect.top;
                this.scrollTop += currentOffset - parseFloat(sidebarScrollOffset);
            }
        } else {
            // scroll sidebar to current active section when navigating via
            // 'next/previous chapter' buttons
            const activeSection = document.querySelector('#mdbook-sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        const sidebarAnchorToggles = document.querySelectorAll('.chapter-fold-toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(el => {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define('mdbook-sidebar-scrollbox', MDBookSidebarScrollbox);


// ---------------------------------------------------------------------------
// Support for dynamically adding headers to the sidebar.

(function() {
    // This is used to detect which direction the page has scrolled since the
    // last scroll event.
    let lastKnownScrollPosition = 0;
    // This is the threshold in px from the top of the screen where it will
    // consider a header the "current" header when scrolling down.
    const defaultDownThreshold = 150;
    // Same as defaultDownThreshold, except when scrolling up.
    const defaultUpThreshold = 300;
    // The threshold is a virtual horizontal line on the screen where it
    // considers the "current" header to be above the line. The threshold is
    // modified dynamically to handle headers that are near the bottom of the
    // screen, and to slightly offset the behavior when scrolling up vs down.
    let threshold = defaultDownThreshold;
    // This is used to disable updates while scrolling. This is needed when
    // clicking the header in the sidebar, which triggers a scroll event. It
    // is somewhat finicky to detect when the scroll has finished, so this
    // uses a relatively dumb system of disabling scroll updates for a short
    // time after the click.
    let disableScroll = false;
    // Array of header elements on the page.
    let headers;
    // Array of li elements that are initially collapsed headers in the sidebar.
    // I'm not sure why eslint seems to have a false positive here.
    // eslint-disable-next-line prefer-const
    let headerToggles = [];
    // This is a debugging tool for the threshold which you can enable in the console.
    let thresholdDebug = false;

    // Updates the threshold based on the scroll position.
    function updateThreshold() {
        const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
        const windowHeight = window.innerHeight;
        const documentHeight = document.documentElement.scrollHeight;

        // The number of pixels below the viewport, at most documentHeight.
        // This is used to push the threshold down to the bottom of the page
        // as the user scrolls towards the bottom.
        const pixelsBelow = Math.max(0, documentHeight - (scrollTop + windowHeight));
        // The number of pixels above the viewport, at least defaultDownThreshold.
        // Similar to pixelsBelow, this is used to push the threshold back towards
        // the top when reaching the top of the page.
        const pixelsAbove = Math.max(0, defaultDownThreshold - scrollTop);
        // How much the threshold should be offset once it gets close to the
        // bottom of the page.
        const bottomAdd = Math.max(0, windowHeight - pixelsBelow - defaultDownThreshold);
        let adjustedBottomAdd = bottomAdd;

        // Adjusts bottomAdd for a small document. The calculation above
        // assumes the document is at least twice the windowheight in size. If
        // it is less than that, then bottomAdd needs to be shrunk
        // proportional to the difference in size.
        if (documentHeight < windowHeight * 2) {
            const maxPixelsBelow = documentHeight - windowHeight;
            const t = 1 - pixelsBelow / Math.max(1, maxPixelsBelow);
            const clamp = Math.max(0, Math.min(1, t));
            adjustedBottomAdd *= clamp;
        }

        let scrollingDown = true;
        if (scrollTop < lastKnownScrollPosition) {
            scrollingDown = false;
        }

        if (scrollingDown) {
            // When scrolling down, move the threshold up towards the default
            // downwards threshold position. If near the bottom of the page,
            // adjustedBottomAdd will offset the threshold towards the bottom
            // of the page.
            const amountScrolledDown = scrollTop - lastKnownScrollPosition;
            const adjustedDefault = defaultDownThreshold + adjustedBottomAdd;
            threshold = Math.max(adjustedDefault, threshold - amountScrolledDown);
        } else {
            // When scrolling up, move the threshold down towards the default
            // upwards threshold position. If near the bottom of the page,
            // quickly transition the threshold back up where it normally
            // belongs.
            const amountScrolledUp = lastKnownScrollPosition - scrollTop;
            const adjustedDefault = defaultUpThreshold - pixelsAbove
                + Math.max(0, adjustedBottomAdd - defaultDownThreshold);
            threshold = Math.min(adjustedDefault, threshold + amountScrolledUp);
        }

        if (documentHeight <= windowHeight) {
            threshold = 0;
        }

        if (thresholdDebug) {
            const id = 'mdbook-threshold-debug-data';
            let data = document.getElementById(id);
            if (data === null) {
                data = document.createElement('div');
                data.id = id;
                data.style.cssText = `
                    position: fixed;
                    top: 50px;
                    right: 10px;
                    background-color: 0xeeeeee;
                    z-index: 9999;
                    pointer-events: none;
                `;
                document.body.appendChild(data);
            }
            data.innerHTML = `
                <table>
                  <tr><td>documentHeight</td><td>${documentHeight.toFixed(1)}</td></tr>
                  <tr><td>windowHeight</td><td>${windowHeight.toFixed(1)}</td></tr>
                  <tr><td>scrollTop</td><td>${scrollTop.toFixed(1)}</td></tr>
                  <tr><td>pixelsAbove</td><td>${pixelsAbove.toFixed(1)}</td></tr>
                  <tr><td>pixelsBelow</td><td>${pixelsBelow.toFixed(1)}</td></tr>
                  <tr><td>bottomAdd</td><td>${bottomAdd.toFixed(1)}</td></tr>
                  <tr><td>adjustedBottomAdd</td><td>${adjustedBottomAdd.toFixed(1)}</td></tr>
                  <tr><td>scrollingDown</td><td>${scrollingDown}</td></tr>
                  <tr><td>threshold</td><td>${threshold.toFixed(1)}</td></tr>
                </table>
            `;
            drawDebugLine();
        }

        lastKnownScrollPosition = scrollTop;
    }

    function drawDebugLine() {
        if (!document.body) {
            return;
        }
        const id = 'mdbook-threshold-debug-line';
        const existingLine = document.getElementById(id);
        if (existingLine) {
            existingLine.remove();
        }
        const line = document.createElement('div');
        line.id = id;
        line.style.cssText = `
            position: fixed;
            top: ${threshold}px;
            left: 0;
            width: 100vw;
            height: 2px;
            background-color: red;
            z-index: 9999;
            pointer-events: none;
        `;
        document.body.appendChild(line);
    }

    function mdbookEnableThresholdDebug() {
        thresholdDebug = true;
        updateThreshold();
        drawDebugLine();
    }

    window.mdbookEnableThresholdDebug = mdbookEnableThresholdDebug;

    // Updates which headers in the sidebar should be expanded. If the current
    // header is inside a collapsed group, then it, and all its parents should
    // be expanded.
    function updateHeaderExpanded(currentA) {
        // Add expanded to all header-item li ancestors.
        let current = currentA.parentElement;
        while (current) {
            if (current.tagName === 'LI' && current.classList.contains('header-item')) {
                current.classList.add('expanded');
            }
            current = current.parentElement;
        }
    }

    // Updates which header is marked as the "current" header in the sidebar.
    // This is done with a virtual Y threshold, where headers at or below
    // that line will be considered the current one.
    function updateCurrentHeader() {
        if (!headers || !headers.length) {
            return;
        }

        // Reset the classes, which will be rebuilt below.
        const els = document.getElementsByClassName('current-header');
        for (const el of els) {
            el.classList.remove('current-header');
        }
        for (const toggle of headerToggles) {
            toggle.classList.remove('expanded');
        }

        // Find the last header that is above the threshold.
        let lastHeader = null;
        for (const header of headers) {
            const rect = header.getBoundingClientRect();
            if (rect.top <= threshold) {
                lastHeader = header;
            } else {
                break;
            }
        }
        if (lastHeader === null) {
            lastHeader = headers[0];
            const rect = lastHeader.getBoundingClientRect();
            const windowHeight = window.innerHeight;
            if (rect.top >= windowHeight) {
                return;
            }
        }

        // Get the anchor in the summary.
        const href = '#' + lastHeader.id;
        const a = [...document.querySelectorAll('.header-in-summary')]
            .find(element => element.getAttribute('href') === href);
        if (!a) {
            return;
        }

        a.classList.add('current-header');

        updateHeaderExpanded(a);
    }

    // Updates which header is "current" based on the threshold line.
    function reloadCurrentHeader() {
        if (disableScroll) {
            return;
        }
        updateThreshold();
        updateCurrentHeader();
    }


    // When clicking on a header in the sidebar, this adjusts the threshold so
    // that it is located next to the header. This is so that header becomes
    // "current".
    function headerThresholdClick(event) {
        // See disableScroll description why this is done.
        disableScroll = true;
        setTimeout(() => {
            disableScroll = false;
        }, 100);
        // requestAnimationFrame is used to delay the update of the "current"
        // header until after the scroll is done, and the header is in the new
        // position.
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                // Closest is needed because if it has child elements like <code>.
                const a = event.target.closest('a');
                const href = a.getAttribute('href');
                const targetId = href.substring(1);
                const targetElement = document.getElementById(targetId);
                if (targetElement) {
                    threshold = targetElement.getBoundingClientRect().bottom;
                    updateCurrentHeader();
                }
            });
        });
    }

    // Takes the nodes from the given head and copies them over to the
    // destination, along with some filtering.
    function filterHeader(source, dest) {
        const clone = source.cloneNode(true);
        clone.querySelectorAll('mark').forEach(mark => {
            mark.replaceWith(...mark.childNodes);
        });
        dest.append(...clone.childNodes);
    }

    // Scans page for headers and adds them to the sidebar.
    document.addEventListener('DOMContentLoaded', function() {
        const activeSection = document.querySelector('#mdbook-sidebar .active');
        if (activeSection === null) {
            return;
        }

        const main = document.getElementsByTagName('main')[0];
        headers = Array.from(main.querySelectorAll('h2, h3, h4, h5, h6'))
            .filter(h => h.id !== '' && h.children.length && h.children[0].tagName === 'A');

        if (headers.length === 0) {
            return;
        }

        // Build a tree of headers in the sidebar.

        const stack = [];

        const firstLevel = parseInt(headers[0].tagName.charAt(1));
        for (let i = 1; i < firstLevel; i++) {
            const ol = document.createElement('ol');
            ol.classList.add('section');
            if (stack.length > 0) {
                stack[stack.length - 1].ol.appendChild(ol);
            }
            stack.push({level: i + 1, ol: ol});
        }

        // The level where it will start folding deeply nested headers.
        const foldLevel = 3;

        for (let i = 0; i < headers.length; i++) {
            const header = headers[i];
            const level = parseInt(header.tagName.charAt(1));

            const currentLevel = stack[stack.length - 1].level;
            if (level > currentLevel) {
                // Begin nesting to this level.
                for (let nextLevel = currentLevel + 1; nextLevel <= level; nextLevel++) {
                    const ol = document.createElement('ol');
                    ol.classList.add('section');
                    const last = stack[stack.length - 1];
                    const lastChild = last.ol.lastChild;
                    // Handle the case where jumping more than one nesting
                    // level, which doesn't have a list item to place this new
                    // list inside of.
                    if (lastChild) {
                        lastChild.appendChild(ol);
                    } else {
                        last.ol.appendChild(ol);
                    }
                    stack.push({level: nextLevel, ol: ol});
                }
            } else if (level < currentLevel) {
                while (stack.length > 1 && stack[stack.length - 1].level > level) {
                    stack.pop();
                }
            }

            const li = document.createElement('li');
            li.classList.add('header-item');
            li.classList.add('expanded');
            if (level < foldLevel) {
                li.classList.add('expanded');
            }
            const span = document.createElement('span');
            span.classList.add('chapter-link-wrapper');
            const a = document.createElement('a');
            span.appendChild(a);
            a.href = '#' + header.id;
            a.classList.add('header-in-summary');
            filterHeader(header.children[0], a);
            a.addEventListener('click', headerThresholdClick);
            const nextHeader = headers[i + 1];
            if (nextHeader !== undefined) {
                const nextLevel = parseInt(nextHeader.tagName.charAt(1));
                if (nextLevel > level && level >= foldLevel) {
                    const toggle = document.createElement('a');
                    toggle.classList.add('chapter-fold-toggle');
                    toggle.classList.add('header-toggle');
                    toggle.addEventListener('click', () => {
                        li.classList.toggle('expanded');
                    });
                    const toggleDiv = document.createElement('div');
                    toggleDiv.textContent = '❱';
                    toggle.appendChild(toggleDiv);
                    span.appendChild(toggle);
                    headerToggles.push(li);
                }
            }
            li.appendChild(span);

            const currentParent = stack[stack.length - 1];
            currentParent.ol.appendChild(li);
        }

        const onThisPage = document.createElement('div');
        onThisPage.classList.add('on-this-page');
        onThisPage.append(stack[0].ol);
        const activeItemSpan = activeSection.parentElement;
        activeItemSpan.after(onThisPage);
    });

    document.addEventListener('DOMContentLoaded', reloadCurrentHeader);
    document.addEventListener('scroll', reloadCurrentHeader, { passive: true });
})();

