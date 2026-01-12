import numpy as np
from rbufrp import BUFRDecoder



with open("/Users/xiang.li1/Downloads/36_2025-12-22T11_00_00.bufr", "rb") as f:
    data = f.read()

decoder = BUFRDecoder()

parsed = decoder.decode(data)

print(parsed.get_message(0))



# parsed = BUFRDecoder.decode(data)