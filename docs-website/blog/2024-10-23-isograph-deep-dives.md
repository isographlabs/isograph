# Deep dives

Want to learn more about Isograph? We recently did some deep dives into how the runtime works and how the compiler works!

<!-- truncate -->

## Runtime deep dive

In the [runtime deep dive](https://www.youtube.com/watch?v=ASIAfEHoU1s), we discussed:

- what is a data driven app? How do changes to subcomponents affect other files? [0:00](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=0s)
- debugging an Isograph app using React DevTools [14:03](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=843s)
- entrypoints and generated queries [17:20](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=1040s)
- client fields (and how Isograph reads the just data selected by a given client field) [20:18](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=1218s)
- what happens when a network request is made? How does Isograph render the entire page? [29:14](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=1754s)
- adding additional logic to GraphQL queries [34:19](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=2059s)
- how do we use the Reader AST to generate the data for a given client field [39:19](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=2359s)
- what about @component client fields? what is the component cache? [41:49](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=2509s)
- how does TypeScript learn about the type of the data parameter passed to client fields? [49:30](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=2970s)
- Loadable fields [51:13](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=3073s)

### Q&A

- do we use the same store for the client component cache and the network response cache? What else might be stored in the network response cache? [1:02:17](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=3737s)
- what does the roadmap look like? How generic will be Isograph in the long term? [1:08:39](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=4119s)
- can we calculate client fields on the server? Benefits in terms of privacy, performance, capabilities, etc. [1:11:50](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=4310s)
- how does one migrate from Redux to Isograph? Modifications to the store? [1:15:17](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=4517s)
- thoughts on a plugin architecture? Yes! [1:17:56](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=4676s)
- other ways to modify fragments [1:21:27](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=4887s)
- what about benchmarks? [1:24:04](https://www.youtube.com/watch?v=ASIAfEHoU1s&t=5044s)

[![Runtime deep dive](https://img.youtube.com/vi/ASIAfEHoU1s/0.jpg)](https://www.youtube.com/watch?v=ASIAfEHoU1s)

## Compiler deep dive

In the [compiler deep dive](https://www.youtube.com/watch?v=w9pLztQD_Ac), we discuss:

- Big picture overview [00:00](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=0s)
- The future direction for the compiler: salsa architecture [10:13](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=613s)
- Most important things that we do [19:08](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=1148s)
- Parsing and processing the GraphQL schema (intro) [20:15](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=1215s)
- Parsing the GraphQL schema [22:15](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=1335s)
- Processing the parsed GraphQL schema [36:05](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=2165s)
- Parsing iso literals [46:50](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=2810s)
- Adding client field literals to the schema [51:10](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=3070s)
- Validating the Isograph schema [53:12](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=3192s)
- Generating the content of generated files [1:01:10](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=3670s)
- Writing artifacts to disk [1:13:09](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=4389s)

### Q&A

- How does watch mode work? [1:14:17](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=4457s)
- What is the future of the compiler? [1:17:20](https://www.youtube.com/watch?v=w9pLztQD_Ac&t=4640s)

[![Compiler deep dive](https://img.youtube.com/vi/w9pLztQD_Ac/0.jpg)](https://www.youtube.com/watch?v=w9pLztQD_Ac)
