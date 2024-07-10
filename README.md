# transport-io
An attempt to implement WebSockets like message framing over WebTransport 
protocol's bidirectional streams

---

## Initial prototype for data frame

Every frame will have the following structure:

 ```
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6
|F| opc | Payload len |    Extended payload length    |
|I| ode |     (7)     |         (16/64)               |
|N|     |             |   (if payload len == 126/127) |
|-----------------------------------------------------|
|    extended payload length if payload len == 127    |
|                                                     |
|                   Payload Data...                   |
```

### FIN: 1 bit
The initial bit indicates if the current fragment is the final frament of the 
message. The first fragment can be the final fragment

### Opcode: 3 bits
The next three bits define the interpretation of payload data.

- %x0 denotes a continuation frame
- %x1 denotes a text frame
- %x2 denotes a binary frame
- %x3-4 are reserved for further non-control frames
- %x5 denotes a discard message
- %x6 is reserved for further control frame

### Payload length: 7 bits, 7+16 bits, 7+64 bits

> [!NOTE]
> basically the same from WebSockets

The next 7 bits define the length of the payload in bytes.
If the number is 0-125 bits in decimal, that would be the payload length for 
the rest of the frame.
If 126, the next 16 bits define the payload length in bytes.
If 127, the next 64 bits define the payload length in bytes.
Note that in all cases, the minimal number of bytes MUST be used to encode the 
length, for example, the length of a 124-byte-long string can't be encoded as 
the sequence 126, 0, 124.

### Application data: x bytes
The application data will have length as specified in the payload length 
section above

