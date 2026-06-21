    #[test]
    fn rime_checksum_computer_matches_librime_crc32_initial_remainder() {
        assert_eq!(rime_checksum_bytes(b"abc"), 0x359a_672f);

        let mut checksum = RimeChecksumComputer::new(0);
        checksum.process_bytes(b"ab");
        checksum.process_bytes(b"c");
        assert_eq!(checksum.checksum(), 0x359a_672f);

        let mut chained = RimeChecksumComputer::new(0x359a_672f);
        chained.process_bytes(b"def");
        assert_eq!(chained.checksum(), 0x050d_415e);
    }

    #[test]
    fn rime_dict_source_checksum_matches_librime_dict_compiler_ordering() {
        let checksum = rime_dict_source_checksum(
            0,
            [b"dict one\n".as_slice(), b"dict two\n".as_slice()],
            Some(b"vocab\n".as_slice()),
        );
        assert_eq!(checksum, 0x0300_9e82);

        let primary = rime_dict_source_checksum(0, [b"primary\n".as_slice()], None);
        let pack = rime_dict_source_checksum(primary, [b"pack\n".as_slice()], None);
        assert_eq!(pack, 0x9024_58b9);

        assert_eq!(
            rime_dict_source_checksum(
                0x1234_5678,
                std::iter::empty::<&[u8]>(),
                Some(b"ignored vocabulary\n".as_slice()),
            ),
            0x1234_5678
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_marks_table_prism_reverse_and_report_statuses() {
        let input = RimeDictRebuildInput {
            source_available: true,
            source_dict_file_checksum: 0x1111_1111,
            pack_source_checksums: Vec::new(),
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(0x1111_1111),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: 0x1111_1111,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(0x1111_1111),
            prebuilt_table_available: false,
            prebuilt_prism_available: false,
            prebuilt_reverse_available: false,
            force_rebuild_table: false,
            force_rebuild_prism: false,
        };
        assert_eq!(
            rime_dict_rebuild_plan(input.clone()),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: false,
                rebuild_reverse: false,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::ReusedFresh,
                    prism: RimeDictArtifactStatus::ReusedFresh,
                    reverse: RimeDictArtifactStatus::ReusedFresh,
                },
            })
        );

        let changed_source = RimeDictRebuildInput {
            source_dict_file_checksum: 0x3333_3333,
            ..input.clone()
        };
        assert_eq!(
            rime_dict_rebuild_plan(changed_source),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x3333_3333,
                rebuild_table: true,
                rebuild_prism: true,
                rebuild_reverse: true,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::Rebuilt,
                    prism: RimeDictArtifactStatus::Rebuilt,
                    reverse: RimeDictArtifactStatus::Rebuilt,
                },
            })
        );

        let changed_schema = RimeDictRebuildInput {
            schema_file_checksum: 0x4444_4444,
            ..input.clone()
        };
        assert_eq!(
            rime_dict_rebuild_plan(changed_schema),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: true,
                rebuild_reverse: false,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::ReusedFresh,
                    prism: RimeDictArtifactStatus::Rebuilt,
                    reverse: RimeDictArtifactStatus::ReusedFresh,
                },
            })
        );

        let stale_reverse = RimeDictRebuildInput {
            reverse_dict_file_checksum: Some(0x5555_5555),
            ..input
        };
        assert_eq!(
            rime_dict_rebuild_plan(stale_reverse),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: true,
                rebuild_prism: false,
                rebuild_reverse: true,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::Rebuilt,
                    prism: RimeDictArtifactStatus::ReusedFresh,
                    reverse: RimeDictArtifactStatus::Rebuilt,
                },
            })
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_reuses_prebuilt_when_source_is_missing() {
        let input = RimeDictRebuildInput {
            source_available: false,
            source_dict_file_checksum: 0,
            pack_source_checksums: Vec::new(),
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(0x1111_1111),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: 0x1111_1111,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(0x1111_1111),
            prebuilt_table_available: true,
            prebuilt_prism_available: true,
            prebuilt_reverse_available: true,
            force_rebuild_table: true,
            force_rebuild_prism: false,
        };
        assert_eq!(
            rime_dict_rebuild_plan(input.clone()),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: false,
                rebuild_reverse: false,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::ReusedPrebuilt,
                    prism: RimeDictArtifactStatus::ReusedPrebuilt,
                    reverse: RimeDictArtifactStatus::ReusedPrebuilt,
                },
            })
        );

        assert_eq!(
            rime_dict_rebuild_plan(RimeDictRebuildInput {
                table_dict_file_checksum: None,
                prebuilt_table_available: false,
                ..input
            }),
            Err(RimeDictRebuildError::MissingSourceAndCompiled)
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_chains_pack_checksums_and_forced_flags() {
        let primary = rime_dict_source_checksum(0, [b"primary\n".as_slice()], None);
        let pack = rime_dict_source_checksum(primary, [b"pack\n".as_slice()], None);
        let input = RimeDictRebuildInput {
            source_available: true,
            source_dict_file_checksum: primary,
            pack_source_checksums: vec![pack],
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(primary),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: primary,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(primary),
            prebuilt_table_available: false,
            prebuilt_prism_available: false,
            prebuilt_reverse_available: false,
            force_rebuild_table: true,
            force_rebuild_prism: true,
        };

        assert_eq!(
            rime_dict_rebuild_plan(input),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: pack,
                rebuild_table: true,
                rebuild_prism: true,
                rebuild_reverse: true,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::Rebuilt,
                    prism: RimeDictArtifactStatus::Rebuilt,
                    reverse: RimeDictArtifactStatus::Rebuilt,
                },
            })
        );
    }
